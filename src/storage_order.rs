#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod router {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Router {
        #[view(getPair)]
        fn get_pair(
            &self,
            first_token_id: TokenIdentifier,
            second_token_id: TokenIdentifier,
        ) -> ManagedAddress;
    }
}

mod pair {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Pair {
        #[view(getAmountIn)]
        fn get_amount_in_view(&self, token_wanted: TokenIdentifier, amount_wanted: BigUint) -> BigUint;
    }
}

use router::ProxyTrait as _;
use pair::ProxyTrait as _;

#[derive(TopEncode)]
pub struct PlaceOrderEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    node: ManagedAddress<M>,
    cid: ManagedBuffer<M>,
    token: TokenIdentifier<M>,
    price: BigUint<M>,
    size: u64,
}

#[elrond_wasm::contract]
pub trait StorageOrder {
    #[proxy]
    fn router_contract_proxy(&self, sc_address: ManagedAddress) -> router::Proxy<Self::Api>;

    #[proxy]
    fn pair_contract_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        cru_token_id: &TokenIdentifier,
        wegld_token_id: &TokenIdentifier,
        router_contract_address: &ManagedAddress,
    ) {
        self.router_contract_address().set(router_contract_address);
        self.cru_token_id().set(cru_token_id);
        self.wegld_token_id().set(wegld_token_id);
        self.supported_tokens().insert(TokenIdentifier::egld());
        self.supported_tokens().insert(wegld_token_id.clone());
        self.supported_tokens().insert(cru_token_id.clone());
    }

    #[only_owner]
    #[endpoint(addSupportedToken)]
    fn add_supported_token(
        &self,
        token_id: TokenIdentifier,
    ) -> SCResult<()> {
        require!(
            !self.supported_tokens().contains(&token_id),
            "Token has been added"
        );
        self.supported_tokens().insert(token_id);

        Ok(())
    }

    #[only_owner]
    #[endpoint(addOrderNode)]
    fn add_order_node(
        &self,
        address: ManagedAddress,
    ) -> SCResult<()> {
        require!(
            !self.order_nodes().contains(&address),
            "Node has been added"
        );
        self.order_nodes().insert(address);

        Ok(())
    }

    #[only_owner]
    #[endpoint(removeSupportedToken)]
    fn remove_supported_token(
        &self,
        token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        require!(
            self.supported_tokens().contains(&token_id),
            "Token not found"
        );
        self.supported_tokens().remove(&token_id);

        Ok(())
    }

    #[only_owner]
    #[endpoint(removeOrderNode)]
    fn remove_order_node(
        &self,
        address: &ManagedAddress,
    ) -> SCResult<()> {
        require!(
            self.order_nodes().contains(&address),
            "Node not found"
        );
        self.order_nodes().remove(&address);

        Ok(())
    }

    #[only_owner]
    #[endpoint(setOrderPrice)]
    fn set_order_price(
        &self,
        base_price: BigUint,
        byte_price: BigUint,
        chain_state_price: BigUint,
    ) -> SCResult<()> {
        self.base_price().set(&base_price);
        self.byte_price().set(&byte_price);
        self.chain_state_price().set(&chain_state_price);

        Ok(())
    }

    #[view(getPrice)]
    fn get_price(
        &self,
        token_id: TokenIdentifier,
        size: u64,
    ) -> BigUint {
        require!(
            !self.base_price().is_empty()
            && !self.byte_price().is_empty()
            && !self.chain_state_price().is_empty(),
            "Order price has not been set"
        );
        let price_in_cru = 
            self.base_price().get()
            + self.byte_price().get().mul(size)
            + self.chain_state_price().get();

        let cru_token_id = self.cru_token_id().get();
        if token_id == cru_token_id.clone() {
            price_in_cru
        } else {
            if token_id == TokenIdentifier::egld() {
                self.get_price_in_token(self.wegld_token_id().get(), price_in_cru)
            } else {
                self.get_price_in_token(token_id, price_in_cru)
            }
        }
    }

    #[payable("*")]
    #[endpoint(placeOrder)]
    fn place_order(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: BigUint,
        cid: ManagedBuffer,
        size: u64,
    ) -> SCResult<()> {
        let node = self.get_random_node();
        self.place_order_with_node(
            payment_token,
            payment_amount,
            node,
            cid,
            size)
    }

    #[payable("*")]
    #[endpoint(placeOrderWithNode)]
    fn place_order_with_node(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: BigUint,
        node_address: ManagedAddress,
        cid: ManagedBuffer,
        size: u64,
    ) -> SCResult<()> {
        let mut real_payment_token = payment_token;
        let mut real_payment_amount = payment_amount;
        let payments = self.call_value().all_esdt_transfers();
        if payments.len() > 0 {
            let real_payments = payments.get(0);
            real_payment_token = real_payments.token_identifier.clone();
            real_payment_amount = real_payments.amount.clone();
        }
        require!(
            self.supported_tokens().contains(&real_payment_token),
            "Unsupported token to pay"
        );
        require!(
            self.order_nodes().contains(&node_address),
            "Unsupported node to order"
        );

        let price = self.get_price(real_payment_token.clone(), size);
        require!(
            real_payment_amount >= price.clone(),
            "Payment amount less than price, please get price again"
        );

        self.send().direct(&node_address, &real_payment_token, 0, &price, b"order successfully");

        let caller = self.blockchain().get_caller();
        if real_payment_amount > price.clone() {
            let change = &real_payment_amount - &price;
            self.send().direct(&caller, &real_payment_token, 0, &change, b"refund change");
        }

        self.emit_place_order_event(
            &caller,
            &node_address,
            &cid,
            &real_payment_token,
            &price,
            size,
        );

        Ok(())
    }

    // private

    fn get_random_node(&self) -> ManagedAddress {
        let nodes = &self.order_nodes();
        require!(
            nodes.len() > 0,
            "No nodes to choose"
        );
        let mut rand_source = RandomnessSource::<Self::Api>::new();
        let rand_index = rand_source.next_usize_in_range(0, nodes.len());
        let mut iter = nodes.iter();
        for _ in 0..rand_index {
            iter.next();
        }
        iter.next().unwrap()
    }

    fn get_price_in_token(
        &self,
        token_id: TokenIdentifier,
        cru_amount: BigUint,
    ) -> BigUint {
        let router_address = self.router_contract_address().get();
        let cru_token_id = self.cru_token_id().get();
        let wegld_token_id = self.wegld_token_id().get();
        let token_cru_pair_address = self.router_contract_proxy(router_address.clone())
            .get_pair(token_id.clone(), cru_token_id.clone())
            .execute_on_dest_context();

        if !token_cru_pair_address.is_zero() {
            self.pair_contract_proxy(token_cru_pair_address)
                .get_amount_in_view(cru_token_id, cru_amount)
                .execute_on_dest_context()
        } else {
            let unit_amount = BigUint::zero() + 1000000000000u64;
            let egld_cru_pair_address = self.router_contract_proxy(router_address.clone())
                .get_pair(wegld_token_id.clone(), cru_token_id.clone())
                .execute_on_dest_context();
            require!(
                !egld_cru_pair_address.is_zero(),
                "Get egld cru swap pair failed"
            );
            let unit_cru_in_egld = self.pair_contract_proxy(egld_cru_pair_address)
                .get_amount_in_view(cru_token_id.clone(), unit_amount.clone())
                .execute_on_dest_context();

            let egld_token_pair_address = self.router_contract_proxy(router_address.clone())
                .get_pair(wegld_token_id.clone(), token_id.clone())
                .execute_on_dest_context();
            require!(
                !egld_token_pair_address.is_zero(),
                "Get egld token swap pair failed"
            );
            let unit_token_in_egld = self.pair_contract_proxy(egld_token_pair_address)
                .get_amount_in_view(token_id.clone(), unit_amount.clone())
                .execute_on_dest_context();

            cru_amount.mul(unit_cru_in_egld).div(unit_token_in_egld)
        }
    }

    fn emit_place_order_event(
        self,
        caller: &ManagedAddress,
        node: &ManagedAddress,
        cid: &ManagedBuffer,
        token: &TokenIdentifier,
        price: &BigUint,
        size: u64,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.place_order_event(
            caller,
            epoch,
            &PlaceOrderEvent {
                caller: caller.clone(),
                node: node.clone(),
                cid: cid.clone(),
                token: token.clone(),
                price: price.clone(),
                size: size,
            },
        )
    }

    // event

    #[event("place_order")]
    fn place_order_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        order_event: &PlaceOrderEvent<Self::Api>,
    );

    // Storage

    #[view(getSupportedTokens)]
    #[storage_mapper("supportedTokens")]
    fn supported_tokens(&self) -> SetMapper<TokenIdentifier>;

    #[view(getOrderNodes)]
    #[storage_mapper("orderNodes")]
    fn order_nodes(&self) -> SetMapper<ManagedAddress>;

    #[view(getBasePrice)]
    #[storage_mapper("basePrice")]
    fn base_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getBytePrice)]
    #[storage_mapper("bytePrice")]
    fn byte_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getChainStatePrice)]
    #[storage_mapper("chainStatePrice")]
    fn chain_state_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getRouterContractAddress)]
    #[storage_mapper("routerContractAddress")]
    fn router_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getCruTokenId)]
    #[storage_mapper("cruTokenId")]
    fn cru_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getWegldTokenId)]
    #[storage_mapper("wegldTokenId")]
    fn wegld_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}

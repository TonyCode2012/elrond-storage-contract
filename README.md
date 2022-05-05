# Storage Order

StorageOrder contract allows users to place a storage order on Elrond Network with EGLD and other ESDT tokens.

## Deployment

Run ***erdpy contract build*** in project root directory to build project.

Configure erdpy.json in project root directory, which looks like:
```
{
    "configurations": {
        "default": {
            "proxy": "https://gateway.elrond.com",
            "chainID": "1"
        }
    },
    "contract": {
        "deploy": {
            "verbose": true,
            "bytecode": "output/storage-order.wasm",
            "recall-nonce": true,
            "pem": "<path_to_your_wallet_pem>",
            "gas-limit": 60000000,
            "arguments": [
                "str:CRU-a5f4aa",
                "str:WEGLD-bd4d79",
                "erd1qqqqqqqqqqqqqpgqq66xk9gfr4esuhem3jru86wg5hvp33a62jps2fy57p"
            ],
            "send": true,
            "outfile": "deploy.interaction.json"
        }
    }
}
```
Note: Use the default value while the pem must be configured to real wallet pem file path.

Run ***erdpy contract deploy*** to deploy contract. Then check result on [elrond explorer](https://explorer.elrond.com/) under account.

## Initialize contract

Now the storage order contract has been deployed successfully. But some operations need to be done before placing order.
Note: these operations can only be called by contract owner.

### Set order price

```
erdpy --verbose contract call <contract_address> --recall-nonce --pem="<path_to_wallet_pem>" --chain="1" --gas-limit="60000000" --function="setOrderPrice" --arguments <base_price> <byte_price> <chain_status_price> --send
```

### Add supported token

```
erdpy --verbose contract call <contract_address> --recall-nonce --pem="<path_to_wallet_pem>" --chain="1" --gas-limit="60000000" --function="addSupportedToken" --arguments <token_identifier> --send
```
Note: removeSupportedToken function can be used to remove token.

### Add order node

```
erdpy --verbose contract call <contract_address> --recall-nonce --pem="<path_to_wallet_pem>" --chain="1" --gas-limit="60000000" --function="addOrderNode" --arguments <node_account> --send
```
Note: removeOrderNode function can be used to remove node.

## Usage

Users can call and query the storage order contract now. Try following command to get price in EGLD with size 262158.
```
erdpy --verbose contract query <contract_address> --function="getPrice" --arguments str:WEGLD-bd4d79 262158
```
After obtaining the price which is 10240000, then use command below to place order:
```
erdpy --verbose contract call <contract_address> --recall-nonce --pem="<path_to_wallet_pem>" --chain="1" --gas-limit="60000000" --function="placeOrder" --arguments str:QmRRAA8bSvQAm8ovK5YUudT1pjiFe2YB6gFKSSyU6GT54B 262158 --value 10240000 --send
```
Note: for pay with other ESDT tokens, please refer to [elrond doc](https://docs.elrond.com/sdk-and-tools/erdjs/erdjs-cookbook/#transfer--execute)

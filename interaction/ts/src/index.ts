import * as fs from "fs";
import * as path from "path";
import {
  Account,
  Address,
  Code,
  CodeMetadata,
  SmartContract,
  TransactionWatcher,
  ResultsParser,
  AddressValue,
  BytesValue,
} from "@elrondnetwork/erdjs";
import { promises } from "fs";
import { ProxyNetworkProvider } from "@elrondnetwork/erdjs-network-providers";
import { UserSecretKey, UserSigner } from "@elrondnetwork/erdjs-walletcore"

const router_address = "erd1qqqqqqqqqqqqqpgqg2esr6d6tfd250x4n3tkhfkw8cc4p2x50n4swatdz6";
const default_token_id = "RIDE-6e4c49";
const provider_url = "https://devnet-api.elrond.com";

async function readTestWalletFileContents(name: string): Promise<string> {
    const filePath = path.join("src", "testutils", "testwallets", name);

    return await fs.promises.readFile(filePath, { encoding: "utf8" });
}

async function loadTestWallet(name: string): Promise<TestWallet> {
    const jsonContents = JSON.parse(await readTestWalletFileContents(name + ".json"));
    const pemContents = await readTestWalletFileContents(name + ".pem");
    const pemKey = UserSecretKey.fromPem(pemContents);
    return new TestWallet(
        new Address(jsonContents.address),
        pemKey.hex(),
        jsonContents,
        pemContents);
}

class TestWallet {
    readonly address: Address;
    readonly secretKeyHex: string;
    readonly secretKey: Buffer;
    readonly signer: UserSigner;
    readonly keyFileObject: any;
    readonly pemFileText: any;
    readonly account: Account;

    constructor(address: Address, secretKeyHex: string, keyFileObject: any, pemFileText: any) {
        this.address = address;
        this.secretKeyHex = secretKeyHex;
        this.secretKey = Buffer.from(secretKeyHex, "hex");
        this.signer = new UserSigner(UserSecretKey.fromString(secretKeyHex));
        this.keyFileObject = keyFileObject;
        this.pemFileText = pemFileText;
        this.account = new Account(this.address);
    }

    getAddress(): Address {
        return this.address;
    }

    async sync(provider: ProxyNetworkProvider) {
        const accountOnNetwork = await provider.getAccount(this.address);
        await this.account.update(accountOnNetwork);
        return this;
    }
}

async function main() {
  // Get network provider
  const networkProvider = new ProxyNetworkProvider(provider_url);

  // Load wallet
  const alice = await loadTestWallet("../../wallet/alice");
  await alice.sync(networkProvider);

  //const aliceOnNetwork = await networkProvider.getAccount(addressOfAlice);
  //alice.account.update(aliceOnNetwork);

  const buffer: Buffer = await promises.readFile("../output/storage-order.wasm");
  const code = Code.fromBuffer(buffer);
  const contract = new SmartContract({});
  const router_sc_address = new Address(router_address);
  const tx = contract.deploy({
      code: code,
      codeMetadata: new CodeMetadata(/* set the parameters accordingly */),
      initArguments: [
        BytesValue.fromUTF8(default_token_id),
        new AddressValue(router_sc_address)],
      gasLimit: 20000000,
      chainID: "D"
  });

  tx.setNonce(alice.account.getNonceThenIncrement());
  await alice.signer.sign(tx);
  alice.account.incrementNonce();

  const contractAddress = SmartContract.computeAddress(tx.getSender(), tx.getNonce());
  await networkProvider.sendTransaction(tx);
  const transactionOnNetwork = await new TransactionWatcher(networkProvider).awaitCompleted(tx);

  //const { returnCode } = new ResultsParser().parseUntypedOutcome(transactionOnNetwork);
  const result = new ResultsParser().parseUntypedOutcome(transactionOnNetwork);
  console.log(result)
  console.log("INFO: deploy storage order contract successfully!");
}

main().catch((e: any) => {
  console.error(e);
})

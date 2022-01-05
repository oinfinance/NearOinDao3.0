const nearAPI = require("near-api-js");
const BN = require("bn.js");

/* 
    buddy.testnet
      private -> ed25519:GtuETJrXTm7aGkwx4yQwa7jLeuiZWQiGxg4WaUjk4AzPcvFVqTyE3c5dsRW8dmbFUcdqp3Xog28owJgk4Ve9hAT
      public  -> ed25519:E8gNM7FnA9uHJFU9t6MuAyKpY8snX1328Upc9VjZEmxf
    
    token1.buddy.testnet
      private -> ed25519:5BqNdEdqGo7DDxoxPj6uo9mxMmu6UPt7DSXdxokFvjhYETqD8SMDUULBRV1xYd6ThmRNe4nMnZwoHk9YdHjSdBRf
      public  -> ed25519:3UKDuJwqrtZByxaKzauCeeQSRyRXY26ybsEaazUkhmWV
 */

const { keyStores, KeyPair, connect, WalletConnection, Contract } = nearAPI;

// TODO 钱包只有浏览器可以使用
/* 
  名称 dev-1618370123280-4818578
   公钥

*/

/* 
  Token
    private ed25519:3zzDtCL1DAWpw3Gzwp2eUbj9d517Mim4Gt5p65fidhwhd5DZEaUBMSSDJfoq29KDMpaHF72dg3LA8gUDSVwquqQ1
    public ed25519:6AUsGgQSSLr1a3SU2oWr1Jw9Dr8N97GXPsWu7ATMqHuF
 */

// TODO 剩下的就是发布一份合约之后，在js端调用

const initFn = async () => {
  const accountName = "buddy.testnet";

  // const KEY_PATH = `~/.near-credentials/default/${accountName}.json`;
  // const keyStore = new keyStores.UnencryptedFileSystemKeyStore(KEY_PATH);

  //  TODO 说白了有私钥，但私钥里面不包含用户名称，但它又是通过用户名称处理的
  //  TODO 所以需要同时传递这2个参数才能判断是哪个用户且有无权限

  const keyStore = new keyStores.InMemoryKeyStore();
  // buddy
  const PRIVATE_KEY =
    "ed25519:GtuETJrXTm7aGkwx4yQwa7jLeuiZWQiGxg4WaUjk4AzPcvFVqTyE3c5dsRW8dmbFUcdqp3Xog28owJgk4Ve9hAT";
  const keyPair = KeyPair.fromString(PRIVATE_KEY);
  await keyStore.setKey("testnet", accountName, keyPair);

  const config = {
    networkId: "testnet",
    keyStore, // optional if not signing transactions
    nodeUrl: "https://rpc.testnet.near.org",
    walletUrl: "https://wallet.testnet.near.org",
    helperUrl: "https://helper.testnet.near.org",
    explorerUrl: "https://explorer.testnet.near.org",
  };
  const near = await connect(config);
  const account = await near.account(accountName);

  // const res = await account.addKey(
  //   "ed25519:CNqF8enjMjAsmDhi37VzXt2RFM2VJH15BsdWMSUQR42z",
  //   "f9353ed8b16b5de4612728cc42e5b12a57be0505.f.ropsten.testnet"
  // );

  // console.log("add-key", res);

  const contract = new Contract(
    account,
    "f9353ed8b16b5de4612728cc42e5b12a57be0505.f.ropsten.testnet",
    {
      viewMethods: ["ft_balance_of"],
      changeMethods: [
        "ft_transfer",
        "internal_register_account",
        "internal_deposit",
        "measure_account_storage_usage",
        "ft_metadata",
        "storage_deposit",
        "storage_balance_of",
      ],
    }
  );

  // console.log(await contract.ft_metadata());
  // console.log(
  //   await contract.storage_deposit(
  //     {
  //       account_id: "token2.buddy.testnet",
  //     },
  //     new BN("300000000000000"),
  //     new BN("900000000000000000000000")
  //   )
  // );

  console.log(
    await contract.storage_balance_of({
      account_id: "token1.buddy.testnet",
    })
  );
};

initFn();

// ed25519:HoZbRhJBZ1q6mqnvJvoz2mbZcqG61X5SH8K5mMYuxhEr

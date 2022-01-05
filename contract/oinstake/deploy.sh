#!/bin/bash
# random
time=`date +%s`
# publisher/contract
# contract_name=$1
export NEAR_ENV=testnet
publisher=$1
default_publisher='oin-finance.testnet'

publisher=${publisher:-$default_publisher}
contract_id="v3.oin-finance.testnet"
# contract_id="oinstake-$time.$publisher"
token_id="meta-v2.pool.testnet"
#token_id="wrap.testnet"
wnear="wrap.testnet"
# oin_id="oin0.shark.testnet"
oin_id="a61175c3dd4bee8a854ffc27c41e39e8e8161d11.factory.goerli.testnet"
pwd_path=`pwd`
#cargo build --target wasm32-unknown-unknown --release
wasm_path="./target/wasm32-unknown-unknown/release/oinstake.wasm"
echo $wasm_path
# create sub-account
near create-account "$contract_id" --initialBalance 12 --masterAccount ${publisher}
# deploy contract

near deploy --accountId "$contract_id" --wasmFile "$wasm_path" --initFunction new --initArgs '{"owner_id":"'${publisher}'"}'
# generate key
res_key=`npx near generate-key`
echo $res_key
public_key=`echo $res_key | egrep -o 'ed25519:[A-Z0-9a-z]{44}'`
echo $public_key

# add key
near add-key "$publisher" "$public_key" --contract-id "$contract_id" --allowance 100
near call  ${contract_id} poke   '{"token_price":"1100000000"}' --account-id ${default_publisher}
near call ${contract_id} register_usdo   '{"account":"'${contract_id}'"}' --account-id  ${contract_id}  --deposit 0.03
near call ${contract_id} register_usdo   '{"account":"'${publisher}'"}' --account-id  ${publisher}  --deposit 0.03
near call ${token_id} storage_deposit   '{"account":"'${contract_id}'"}' --account-id  ${contract_id}  --deposit 0.03
near call ${wnear} storage_deposit   '{"account":"'${contract_id}'"}' --account-id  ${contract_id}  --deposit 0.03
near call ${oin_id} storage_deposit   '{"account":"'${contract_id}'"}' --account-id  ${contract_id}  --deposit 0.03
near call ${contract_id} add_white   '{"account":"zxzzx.testnet"}' --account-id  ${publisher} 
near call ${contract_id} add_white   '{"account":"test33.testnet"}' --account-id  ${publisher} 
near call ${contract_id} add_mul_white   '{"account":"'${publisher}'"}' --account-id  ${publisher} 
# near call ${contract_id} add_mul_white   '{"account":"zxzzx.testnet"}' --account-id  ${publisher} 
# near call ${contract_id} add_mul_white   '{"account":"shark.testnet"}' --account-id  ${publisher}
near call ${contract_id} add_mul_white   '{"account":"test33.testnet"}' --account-id  ${publisher} 
# near call ${contract_id} add_mul_white   '{"account":"test104.testnet"}' --account-id  ${publisher}
near call ${contract_id} add_mul_white   '{"account":"shakou.testnet"}' --account-id  ${publisher} 
near call ${contract_id} add_white   '{"account":"shark.testnet"}' --account-id  ${publisher} 
# near call ${contract_id} add_reward_coin '{"token":"'${wnear}'","reward_speed":"100000000000000000000000","double_scale":"1000000000000000000000000"}' --account-id ${publisher} 
echo
echo "Contract:$contract_id published successfully."

#redeploy
contract_id="v8.test22.testnet"
# cargo build --target wasm32-unknown-unknown --release
wasm_path="./oinstake.wasm"
# wasm_path="./target/wasm32-unknown-unknown/release/oinstake.wasm"
near deploy --accountId "$contract_id" --wasmFile "$wasm_path" 
y
echo " redeply Contract:$contract_id  successfully."
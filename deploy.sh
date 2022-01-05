#!/bin/bash

if [ -z "$1" ]
then
    echo "Error: no-exist contract_name"
    exit 1
fi

# random
time=`date +%s`

# publisher/contract
contract_name=$1
publisher=$2
default_publisher='buddy.testnet'
publisher=${publisher:-$default_publisher}

contract_id="$contract_name-$time.$publisher"
pwd_path=`pwd`

wasm_path="$pwd_path/contract/$contract_name/target/wasm32-unknown-unknown/release/$contract_name.wasm"

echo $wasm_path

# create sub-account
npx near create-account "$contract_id" --initialBalance 2 --masterAccount buddy.testnet

# deploy contract
npx near deploy --accountId "$contract_id" --wasmFile "$wasm_path" --initFunction new --initArgs '{}'

# generate key
res_key=`npx near generate-key`
echo $res_key
public_key=`echo $res_key | egrep -o 'ed25519:[A-Z0-9a-z]{44}'`
echo $public_key

# add key
npx near add-key "$publisher" "$public_key" --contract-id "$contract_id" --allowance 100


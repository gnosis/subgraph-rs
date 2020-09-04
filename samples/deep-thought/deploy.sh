#!/usr/bin/env bash

set -e

ROOT=$(dirname "${BASH_SOURCE[0]}")

GANACHE="http://127.0.0.1:8545"
GRAPH="http://127.0.0.1:8020"
IPFS="http://127.0.0.1:5001"
SUBGRAPH="subgraph-rs/deep-thought"

log() {
	1>&2 echo $1
}

jsonrpc() {
	curl -s -X POST $1 \
		-H 'Content-Type: application/json; charset=utf-8' \
		--data-binary @-
}

deploy_contract() {
	log "Deploying contract..."

	bin=$(cat "$ROOT/DeepThought/bin/DeepThought.bin")
	tx=$(jsonrpc $GANACHE << JSON | jq -r '.result'
{
	"jsonrpc": "2.0",
	"method": "eth_sendTransaction",
	"params": [{
		"from": "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1",
		"gas": "0x100000",
		"data": "0x$bin"
	}]
}
JSON
	)
	
	contract=$(jsonrpc $GANACHE << JSON | jq -r '.result.contractAddress'
{
	"jsonrpc": "2.0",
	"method": "eth_getTransactionReceipt",
	"params": ["$tx"]
}
JSON
	)

	log "Contract transaction: '$tx'"
	log "Contract address:     '$contract'"
	echo $contract
}

create_subgraph() {
	log "Creating subgraph '$SUBGRAPH'..."
	jsonrpc $GRAPH << JSON > /dev/null
{
	"jsonrpc": "2.0",
	"method": "subgraph_create",
	"params": { "name": "$SUBGRAPH" },
	"id": "1"
}
JSON
}

ipfs_upload() {
	log "Uploading '$1' to IPFS..."

	if [[ -z "$2" ]]; then
		hash=$(curl -s -F "file=@$ROOT/$1;filename=$1" $IPFS/api/v0/add | jq -r '.Hash' | head -n 1)
	else
		hash=$(curl -s -F "file=@-;filename=$1" $IPFS/api/v0/add <<< "$2" | jq -r '.Hash' | head -n 1)
	fi
	curl -s -X POST "http://localhost:5001/api/v0/pin/add?arg=$hash" > /dev/null

	log "IPFS hash '$hash'"
	echo $hash
}

deploy_subgraph() {
	log "Deploying subgraph '$SUBGRAPH'..."

	subgraph=$(cat "$ROOT/subgraph.yaml" | sed 's/address: .*/address: "'$1'"/')
	while read line; do
		indent=$(echo "$line" | sed 's/^#\(\s*\)f.*$/\1/')
		file=$(echo "$line" | sed 's/.*file:\s*\(.*\)/\1/')
		hash=$(ipfs_upload "$file")

		escaped_file=${file//\//\\\/}
		subgraph=$(echo "$subgraph" | sed "s/file: $escaped_file/file:\\n$indent  \\/: \\/ipfs\\/$hash/")
	done <<< $(echo "$subgraph" | grep '^\s*file:' | sed 's/^/#/')

	hash=$(ipfs_upload "subgraph.yaml" "$subgraph")
	playground=$(jsonrpc $GRAPH << JSON | jq -r '.result.playground'
{
	"jsonrpc": "2.0",
	"method": "subgraph_deploy",
	"params": {
		"name": "$SUBGRAPH",
		"ipfs_hash": "$hash"
	},
	"id": "2"
}
JSON
)
	echo "http://127.0.0.1$playground"
}

contract_address=$(deploy_contract)
create_subgraph
deploy_subgraph $contract_address

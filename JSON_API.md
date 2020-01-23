### Block by height
This will return the genesis block, automatically generated in this version on start.

```
{
    "method": "block_by_height",
    "params": 
		{ "height" : 0 },
    "jsonrpc": "2.0",
    "id": 4444
}
```


### Block by hash
The block by this hash does not exist but if it did, it would the block.

```
{
    "method": "block_by_hash",
    "params": 
		{ "hash" : "540cd46eae4eb6e76eb905537d785f3bf99e6c30841dfe460cc0052a76ec8910"},
    "jsonrpc": "2.0",
    "id": 4444
}
```

### Get transaction
The transaction by this hash does not exist but if it did, it would the transaction.

```
{
    "method": "get_transaction",
    "params": 
		{ "hash" : "540cd46eae4eb6e76eb905537d785f3bf99e6c30841dfe460cc0052a76ec8910"},
    "jsonrpc": "2.0",
    "id": 4444
}
```

### Get account
Returns the count of "system transactions".
Zero hash receives all transactions created through terminal input.

```
{
    "method": "get_account",
    "params": 
		{ "hash" : "0000000000000000000000000000000000000000000000000000000000000000"},
    "jsonrpc": "2.0",
    "id": 4444
}
```


### Publish Transaction
Have the node sign a byte vector <data> 
and publish it through the network to <to> account as recipient.

```
{
    "method": "publish_transaction",
    "params": 
	{
        "to" : [1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2],
		"data": [1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,7]
    },
    "jsonrpc": "2.0",
    "id": 4444
}
``` 


### Publish Raw Transaction
Present method publishes an already signed transaction through the network.

```
{
	"method": "publish_raw_transaction",
	"params": {
		"tx": {
			"pubkey": [143,122,144,250,32,214,217,172,112,254,96,166,93,211,206,49,222,194,50,157,156,173,191,243,117,18,28,151,20,220,255,120],
			"sig": [
				215,24,82,56,179,21,20,124,85,36,208,243,106,225,75,156,65,97,126,17,202,194,25,44,166,200,101,93,111,181,59,80,209,35,101,54,88,250,121,127,175,226,23,143,210,80,211,118,58,159,23,211,1,254,76,254,112,71,235,127,150,99,148,10
			],
			"transaction": {
				"data": [1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,7],
				"nonce": 15235349517107540267,
				"recipient": [1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2],
				"timestamp": 1579260589
			}
		}
	},
	"jsonrpc": "2.0",
	"id": 4444
}
```
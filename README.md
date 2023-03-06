![basic workflow](https://github.com/jacksoom/candle-auction/actions/workflows/Basic.yml/badge.svg)
![test workflow](https://github.com/jacksoom/candle-auction/actions/workflows/Test.yml/badge.svg)
[![codecov](https://codecov.io/gh/jacksoom/candle-auction/branch/main/graph/badge.svg)](https://codecov.io/gh/jacksoom/candle-auction)
# 🕯️ Candle Auctions on CosmWasm! 🎃
This is an [CosmWasm](https://cosmwasm.com/) smartcontract implementing a [candle auction](https://en.wikipedia.org/wiki/Candle_auction) logic.

With this contract, one can set up a candle auction for a **NFT collection** or a **domain name**!  

## Design details
### 1: Build a auction
Everyone can calling Auction message. Then before the auction has started, transfer the nft to be auctioned into the contract and add it, and add the callback message {id: $[auction_id]},

### 2: Auction bid
 During the duration of the auction, the bidder can bid(CW20 callback or ```BidForDenom```), but the bid must be greater than the previous bid
### 3: Candle blow
After the auction, Anyone can blowing out the auction candle. The contract will call the random number of the external oracle to confirm auction end time.
```
end_time = auction_start_time+ random_num % auction_duration
```
The auction winner is the one with the highest bid less than the end time 
- Auction1: refunds for non-winners.
- Auction2: Transfer nft to winner.
- Auction3: Transfer bid currency to seller.

### 4: Advantage
- Support multiple nft auctions at one time.
- Support multiple payment. denom/cw20.


## Build
1: Run check and test
```sh
make all
```

2: Build
```
sh optimize
```

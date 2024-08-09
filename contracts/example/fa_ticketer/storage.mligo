// MIT License

// Copyright (c) 2024 Baking Bad

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#import "../../common/tokens/tokens.mligo" "Token"
#import "../../common/types/ticket.mligo" "Ticket"
#import "../../common/errors.mligo" "Errors"


(*
    Ticketer storage type:
    - metadata: a big_map containing the metadata of the contract (TZIP-016), immutable
    - token: a token which Ticketer accepts for minting tickets, immutable
    - content: a content of the ticket to be minted, immutable
    - total_supply: a total supply of the ticket, the initial value should be 0
*)
type t = {
    metadata : (string, bytes) big_map;
    token : Token.t;
    content : Ticket.content_t;
    total_supply : nat;
}

(* The maximum amount of ticket which can be stored on the L2 side is 2^256-1 *)
let two_to_the_256th = 115_792_089_237_316_195_423_570_985_008_687_907_853_269_984_665_640_564_039_457_584_007_913_129_639_936n

let increase_total_supply (amount : nat) (store : t) : t =
    let total_supply = store.total_supply + amount in
    if total_supply >= two_to_the_256th
        then failwith Errors.total_supply_exceed_max
        else { store with total_supply }

let decrease_total_supply (amount : nat) (store : t) : t =
    let total_supply = if amount > store.total_supply
        then failwith Errors.total_supply_exceeded
        else abs (store.total_supply - amount) in
    { store with total_supply }

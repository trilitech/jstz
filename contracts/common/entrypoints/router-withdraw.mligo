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

#import "../types/ticket.mligo" "Ticket"
#import "../errors.mligo" "Errors"


(*
    `router-withdraw` is router interface that used for redirecting
    tickets during withdrawal from a rollup:

    - receiver: an address which will receive the unlocked token.
    - ticket: provided ticket to be burned.
 *)
type t = [@layout:comb] {
    receiver: address;
    ticket: Ticket.t;
}

let get (router : address) : t contract =
    match Tezos.get_entrypoint_opt "%withdraw" router with
    | None -> failwith(Errors.router_entrypoint_not_found)
    | Some entry -> entry

let send
        (router : address)
        (params : t)
        : operation =
    let entry = get router in
    Tezos.transaction params 0mutez entry

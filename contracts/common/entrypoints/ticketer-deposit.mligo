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

#import "../errors.mligo" "Errors"


(*
    `ticketer-deposit` is deposit interface for the Ticketer contract
    which represents the amount of token to be wrapped to the ticket
*)
type t = nat

let get (router : address) : t contract =
    match Tezos.get_entrypoint_opt "%deposit" router with
    | None -> failwith(Errors.ticketer_deposit_not_found)
    | Some entry -> entry

let send
        (ticketer : address)
        (amount : t)
        : operation =
    let entry = get ticketer in
    Tezos.transaction amount 0mutez entry

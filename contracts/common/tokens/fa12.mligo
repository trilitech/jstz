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

type t = address

type transfer_params = [@layout:comb] {
    [@annot:from] from_: address;
    [@annot:to] to_: address;
    value: nat;
}

let get_transfer (token_address: address) : transfer_params contract =
    match Tezos.get_entrypoint_opt "%transfer" token_address with
    | None -> failwith Errors.invalid_fa12
    | Some entry -> entry

let send_transfer
        (from_: address)
        (to_: address)
        (token_address: address)
        (value: nat)
        : operation =
    let params = { from_; to_; value } in
    let entry = get_transfer token_address in
    Tezos.transaction params 0mutez entry

type approve_params = [@layout:comb] {
    spender: address;
    value: nat;
}

let get_approve (contract_address: address) : approve_params contract =
    match Tezos.get_entrypoint_opt "%approve" contract_address with
    | None -> failwith Errors.invalid_fa12
    | Some entry -> entry

let send_approve
        (contract_address: address)
        (spender: address)
        (value: nat)
        : operation =
    let params = { spender; value } in
    let entry = get_approve contract_address in
    Tezos.transaction params 0mutez entry

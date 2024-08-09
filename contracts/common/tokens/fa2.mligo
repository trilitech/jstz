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

type t = address * nat

type transfer_txs_item = [@layout:comb] {
    to_: address;
    token_id: nat;
    amount: nat;
}

type transfer_txs = transfer_txs_item list

type transfer_params = {
    from_: address;
    txs: transfer_txs;
} list

let get_transfer (address : address) : transfer_params contract =
    match Tezos.get_entrypoint_opt "%transfer" address with
    | None -> failwith Errors.invalid_fa2
    | Some entry -> entry

let send_transfer
        (from_: address)
        (token_address: address)
        (txs: transfer_txs)
        : operation =
    let params = [{ from_; txs }] in
    let entry = get_transfer token_address in
    Tezos.transaction params 0mutez entry

type operator_param_t = [@layout:comb] {
    owner: address;
    operator: address;
    token_id: nat;
}

type update_operator_param_t = [@layout:comb]
    | Add_operator of operator_param_t
    | Remove_operator of operator_param_t

type update_operator_params_t = update_operator_param_t list

let get_approve (address : address) : update_operator_params_t contract =
    match Tezos.get_entrypoint_opt "%update_operators" address with
    | None -> failwith Errors.invalid_fa2
    | Some entry -> entry

let send_approve
        (contract_address: address)
        (token_id: nat)
        (operator: address)
        : operation =
    let owner = Tezos.get_self_address () in
    let operator_param = { operator; token_id; owner } in
    let params = [Add_operator(operator_param)] in
    let entry = get_approve contract_address in
    Tezos.transaction params 0mutez entry

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
    Tools for working with tickets
*)
type content_t = nat * bytes option
type t = content_t ticket


let create
        (content : content_t)
        (amount : nat)
        : t =
    match Tezos.create_ticket content amount with
    | None -> failwith Errors.ticket_creation_failed
    | Some t -> t

let get
        (address : address)
        : t contract =
    match Tezos.get_contract_opt address with
    | None -> failwith Errors.failed_to_get_ticket_entrypoint
    | Some c -> c

let split
        (ticket : t)
        (split_amount : nat)
        : t * t =
    (* Splits ticket into two tickets with given amounts *)
    let (_, (_, amount)), ticket = Tezos.read_ticket ticket in
    let keep_amount =
        if amount >= split_amount then abs(amount - split_amount)
        else failwith Errors.insufficient_amount in
    match Tezos.split_ticket ticket (split_amount, keep_amount) with
    | Some split_tickets -> split_tickets
    | None -> failwith Errors.irreducible_amount

let send
        (ticket : t)
        (receiver : address)
        : operation =
    let receiver_contract = get receiver in
    Tezos.transaction ticket 0mutez receiver_contract

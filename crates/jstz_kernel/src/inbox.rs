use jstz_crypto::public_key_hash::PublicKeyHash;
use jstz_proto::operation::{external::Deposit, ExternalOperation, SignedOperation};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use tezos_crypto_rs::hash::ContractKt1Hash;
use tezos_smart_rollup::inbox::ExternalMessageFrame;
use tezos_smart_rollup::michelson::ticket::FA2_1Ticket;
use tezos_smart_rollup::michelson::{
    MichelsonBytes, MichelsonContract, MichelsonNat, MichelsonOption,
};
use tezos_smart_rollup::{
    inbox::{InboxMessage, InternalInboxMessage, Transfer},
    michelson::MichelsonPair,
    prelude::{debug_msg, Runtime},
    types::Contract,
};

pub type ExternalMessage = SignedOperation;
pub type InternalMessage = ExternalOperation;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Message {
    External(ExternalMessage),
    Internal(InternalMessage),
}

// reciever, ticket
pub type RollupType = MichelsonPair<MichelsonContract, FA2_1Ticket>;

const NATIVE_TICKET_ID: u32 = 0_u32;
const NATIVE_TICKET_CONTENT: MichelsonOption<MichelsonBytes> = MichelsonOption(None);

fn is_valid_native_deposit(
    rt: &mut impl Runtime,
    ticket: &FA2_1Ticket,
    native_ticketer: &ContractKt1Hash,
) -> bool {
    let creator = ticket.creator();
    let contents = ticket.contents();
    match &creator.0 {
        Contract::Originated(kt1) if kt1 == native_ticketer => (),
        _ => {
            debug_msg!(rt, "Deposit ignored because of different ticketer");
            return false;
        }
    };

    let native_ticket_id = MichelsonNat::from(NATIVE_TICKET_ID);
    if contents.0 != native_ticket_id {
        debug_msg!(rt, "Deposit ignored because of different ticket id");
        return false;
    }

    if contents.1 != NATIVE_TICKET_CONTENT {
        debug_msg!(rt, "Deposit ignored because of different ticket content");
        return false;
    }

    true
}

fn read_transfer(
    rt: &mut impl Runtime,
    transfer: Transfer<RollupType>,
    ticketer: &ContractKt1Hash,
) -> Option<Message> {
    debug_msg!(rt, "Internal message: transfer\n");

    let ticket = transfer.payload.1;

    if is_valid_native_deposit(rt, &ticket, ticketer) {
        let amount = ticket.amount().to_u64()?;
        let pkh = transfer.payload.0 .0.to_b58check();
        let reciever = PublicKeyHash::from_base58(&pkh).ok()?;
        let content = Deposit { amount, reciever };
        debug_msg!(rt, "Deposit: {content:?}\n");
        Some(Message::Internal(InternalMessage::Deposit(content)))
    } else {
        None
    }
}

fn read_external_message(rt: &mut impl Runtime, bytes: &[u8]) -> Option<ExternalMessage> {
    let msg: ExternalMessage = bincode::deserialize(bytes).ok()?;
    debug_msg!(rt, "External message: {msg:?}\n");
    Some(msg)
}

pub fn read_message(rt: &mut impl Runtime, ticketer: ContractKt1Hash) -> Option<Message> {
    let input = rt.read_input().ok()??;
    let _ = rt.mark_for_reboot();

    let (_, message) = InboxMessage::<RollupType>::parse(input.as_ref()).ok()?;

    match message {
        InboxMessage::Internal(InternalInboxMessage::StartOfLevel) => {
            // Start of level message pushed by the Layer 1 at the
            // beginning of eavh level.
            debug_msg!(rt, "Internal message: start of level\n");
            None
        }
        InboxMessage::Internal(InternalInboxMessage::InfoPerLevel(info)) => {
            // The "Info per level" messages follows the "Start of level"
            // message and contains information on the previous Layer 1 block.
            debug_msg!(
                rt,
                "Internal message: level info \
                        (block predecessor: {}, predecessor_timestamp: {}\n",
                info.predecessor,
                info.predecessor_timestamp
            );
            None
        }
        InboxMessage::Internal(InternalInboxMessage::EndOfLevel) => {
            // The "End of level" message is pushed by the Layer 1
            // at the end of each level.
            debug_msg!(rt, "Internal message: end of level\n");
            None
        }
        InboxMessage::Internal(InternalInboxMessage::Transfer(transfer)) => {
            if transfer.destination.hash().as_ref()
                != &rt.reveal_metadata().raw_rollup_address
            {
                debug_msg!(
                    rt,
                    "Internal message ignored because of different smart rollup address"
                );
                return None;
            };
            read_transfer(rt, transfer, &ticketer)
        }
        InboxMessage::External(bytes) => match ExternalMessageFrame::parse(bytes) {
            Ok(frame) => match frame {
                ExternalMessageFrame::Targetted { address, contents } => {
                    let metadata = rt.reveal_metadata();
                    let rollup_address = metadata.address();
                    if &rollup_address != address.hash() {
                        debug_msg!(
                          rt,
                            "Skipping message: External message targets another rollup. Expected: {}. Found: {}\n",
                            rollup_address,
                            address.hash()
                        );
                        None
                    } else {
                        match read_external_message(rt, contents) {
                            Some(msg) => Some(Message::External(msg)),
                            None => {
                                debug_msg!(rt, "Failed to parse the external message\n");
                                None
                            }
                        }
                    }
                }
            },
            Err(_) => {
                debug_msg!(rt, "Failed to parse the external message frame\n");
                None
            }
        },
    }
}

#[cfg(test)]
mod test {
    use jstz_mock::mock::{JstzMockHost, MockNativeDeposit};
    use jstz_proto::operation::external;
    use tezos_crypto_rs::hash::{ContractKt1Hash, HashTrait};
    use tezos_smart_rollup::types::SmartRollupAddress;

    use super::{read_message, InternalMessage, Message};

    #[test]
    fn read_message_ignored_on_different_smart_rollup_address() {
        let mut host = JstzMockHost::new(true);
        let alternative_smart_rollup_address =
            SmartRollupAddress::from_b58check("sr1Ghq66tYK9y3r8CC1Tf8i8m5nxh8nTvZEf")
                .unwrap();
        let deposit = MockNativeDeposit {
            smart_rollup: Some(alternative_smart_rollup_address),
            ..MockNativeDeposit::default()
        };
        host.add_deposit_message(&deposit);
        let ticketer = host.get_ticketer();
        let result = read_message(host.rt(), ticketer);
        assert_eq!(result, None)
    }

    #[test]
    fn read_message_native_deposit_succeeds() {
        let mut host = JstzMockHost::new(true);
        let deposit = MockNativeDeposit::default();
        let ticketer = host.get_ticketer();
        host.add_deposit_message(&deposit);
        if let Message::Internal(InternalMessage::Deposit(external::Deposit {
            amount,
            reciever,
        })) =
            read_message(host.rt(), ticketer).expect("Expected message but non received")
        {
            assert_eq!(amount, 100);
            assert_eq!(reciever.to_base58(), deposit.receiver.to_b58check())
        } else {
            panic!("Expected deposit message")
        }
    }

    #[test]
    fn read_message_native_deposit_ignored_different_ticketer() {
        let mut host = JstzMockHost::new(true);
        let ticketer = host.get_ticketer();
        let deposit = MockNativeDeposit {
            ticketer: ContractKt1Hash::from_b58check(
                "KT1KRj5VMNmhxobTJBPq7u2kacqbxu9Cntx6",
            )
            .unwrap(),
            ..MockNativeDeposit::default()
        };
        host.add_deposit_message(&deposit);
        assert_eq!(read_message(host.rt(), ticketer), None);
    }

    #[test]
    fn read_message_native_deposit_ignored_different_ticket_id() {
        let mut host = JstzMockHost::new(true);
        let ticketer = host.get_ticketer();
        let deposit = MockNativeDeposit {
            ticket_content: (1, None),
            ..MockNativeDeposit::default()
        };
        host.add_deposit_message(&deposit);
        assert_eq!(read_message(host.rt(), ticketer), None);
    }

    #[test]
    fn read_message_native_deposit_ignored_different_ticket_value() {
        let mut host = JstzMockHost::new(true);
        let ticketer = host.get_ticketer();
        let deposit = MockNativeDeposit {
            ticket_content: (0, Some(b"1234".to_vec())),
            ..MockNativeDeposit::default()
        };
        host.add_deposit_message(&deposit);
        assert_eq!(read_message(host.rt(), ticketer), None);
    }
}

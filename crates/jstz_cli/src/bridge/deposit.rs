use log::{debug, info};

use crate::{
    config::{Config, NetworkName},
    error::Result,
    utils::AddressOrAlias,
};

pub fn exec(
    from: String,
    to: AddressOrAlias,
    amount: u64,
    network: Option<NetworkName>,
) -> Result<()> {
    let cfg = Config::load()?;

    let to = to.resolve(&cfg)?;
    debug!("resolved `to` -> {:?}", to);

    // 2. Execute the octez-client command
    cfg.octez_client(&network)?.call_contract(
        &from,
        "jstz_bridge",
        "deposit",
        &format!("(Pair {} 0x{})", amount, hex::encode_upper(to.as_bytes())),
    )?;

    info!("Deposited {} CTEZ to {}", amount, to);

    Ok(())
}

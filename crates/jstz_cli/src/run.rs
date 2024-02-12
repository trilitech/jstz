use std::str::FromStr;

use anyhow::bail;
use http::{HeaderMap, Method, Uri};
use jstz_proto::{
    operation::{Content as OperationContent, Operation, RunContract, SignedOperation},
    receipt::Content as ReceiptContent,
};
use log::{debug, info};
use url::Url;

use crate::{
    config::Config,
    error::{anyhow, bail_user_error, user_error, Result},
    term::styles,
    utils::{read_file_or_input_or_piped, AddressOrAlias},
};

pub const DEFAULT_GAS_LIMIT: u32 = 100_000;

pub async fn exec(
    url: String,
    http_method: String,
    gas_limit: u32,
    json_data: Option<String>,
) -> Result<()> {
    let cfg = Config::load()?;

    // 1. Get the current user (checking if we are logged in)
    let (_, user) = cfg.accounts.current_user().ok_or(user_error!(
        "You are not logged in. Please run `jstz login`."
    ))?;

    let jstz_client = cfg.jstz_client()?;

    // 2. Resolve the URL
    let mut url_object = Url::parse(&url)
        .map_err(|_| user_error!("Invalid URL {}.", styles::url(&url)))?;

    if let Some(host) = url_object.host_str() {
        let address_or_alias = AddressOrAlias::from_str(host)?;

        if address_or_alias.is_alias() {
            info!("Resolving host '{}'...", host);

            let address = address_or_alias.resolve(&cfg)?;

            info!("Resolved host '{}' to '{}'.", host, address);

            url_object
                .set_host(Some(&address.to_string()))
                .map_err(|_| anyhow!("Failed to set host"))?;
        }
    } else {
        bail_user_error!("URL {} requires a host.", styles::url(&url));
    }

    debug!("Resolved URL: {}", url_object.to_string());

    // 3. Construct the signed operation
    let nonce = jstz_client.get_nonce(&user.address).await?;

    // SAFETY: `url` is a valid URI since URLs are a subset of  URIs and `url_object` is a valid URL.
    let url: Uri = url_object
        .to_string()
        .parse()
        .expect("`url_object` is an invalid URL.");

    let method = Method::from_str(&http_method)
        .map_err(|_| user_error!("Invalid HTTP method: {}", http_method))?;

    debug!("Method: {:?}", method);

    let body = read_file_or_input_or_piped(json_data)?.map(String::into_bytes);

    debug!("Body: {:?}", body);

    let op = Operation {
        source: user.address.clone(),
        nonce,
        content: OperationContent::RunContract(RunContract {
            uri: url,
            method,
            headers: HeaderMap::default(),
            body,
            gas_limit: gas_limit
                .try_into()
                .map_err(|_| anyhow!("Invalid gas limit."))?,
        }),
    };

    debug!("Operation: {:?}", op);

    let hash = op.hash();

    debug!("Operation hash: {}", hash.to_string());

    let signed_op =
        SignedOperation::new(user.public_key.clone(), user.secret_key.sign(&hash)?, op);

    debug!("Signed operation: {:?}", signed_op);

    // 4. Send message to jstz node
    info!(
        "Running function at {}...",
        styles::url(&url_object.to_string())
    );

    jstz_client.post_operation(&signed_op).await?;
    let receipt = jstz_client.wait_for_operation_receipt(&hash).await?;

    debug!("Receipt: {:?}", receipt);
    let (status_code, headers, body) = match receipt.inner {
        Ok(ReceiptContent::RunContract(run_contract)) => (
            run_contract.status_code,
            run_contract.headers,
            run_contract.body,
        ),

        Ok(_) => bail!("Expected a `RunContract` receipt, but got something else."),
        Err(err) => bail!("Contract failed with error {err:?}"),
    };

    info!("Status code: {}", status_code);
    info!("Headers: {:?}", headers);
    if let Some(body) = body {
        info!("Body: {}", String::from_utf8_lossy(&body));
    }

    cfg.save()?;

    Ok(())
}

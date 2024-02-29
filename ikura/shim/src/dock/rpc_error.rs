use jsonrpsee::types::error::ErrorObjectOwned;

pub fn no_signing_key() -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INTERNAL_ERROR_CODE,
        "Internal Error: no key for signing blobs",
        None::<()>,
    )
}

pub fn nonce_obtain_error(e: anyhow::Error) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INTERNAL_ERROR_CODE,
        format!("Internal Error: failed to obtain nonce: {:?}", e),
        None::<()>,
    )
}

pub fn submit_extrinsic_error(e: anyhow::Error) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INTERNAL_ERROR_CODE,
        format!(
            "Internal Error: failed to create a submit blob extrinsic: {:?}",
            e
        ),
        None::<()>,
    )
}

pub fn submission_error(e: anyhow::Error) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INTERNAL_ERROR_CODE,
        format!("Internal Error: failed to submit blob: {:?}", e),
        None::<()>,
    )
}

use jsonrpsee::types::error::ErrorObjectOwned;

pub fn bad_namespace() -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INVALID_PARAMS_CODE,
        "Invalid namespace",
        None::<()>,
    )
}

pub fn no_signing_key() -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        jsonrpsee::types::error::INTERNAL_ERROR_CODE,
        "Internal Error: no key for signing blobs",
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

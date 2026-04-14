use crate::error::{Error, Result};

pub(crate) fn ensure_success_response(response: rquest::Response) -> Result<rquest::Response> {
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(Error::AuthenticationFailed);
    }

    response.error_for_status().map_err(|err| Error::Server {
        status: err.status().map_or(500, |status| status.as_u16()),
        message: err.to_string(),
    })
}

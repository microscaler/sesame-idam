use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::check_export_status::{Request, Response};

/// Handler for Check Export Status
#[handler(CheckExportStatusController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    Response {
        export_id: req.data.export_id,
        status: "pending".to_string(),
        download_url: None,
        estimated_completion: None,
    }
}

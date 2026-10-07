//! OpenAPI-to-PDS conversion for data-product request DTOs.

use kiva_pds::{
    DataProductReadIntent as PdsDataProductReadIntent,
    DataProductRegistrationCommandInput as PdsDataProductRegistrationCommandInput,
};
use kiva_pds_openapi_contract as pds_openapi;

use super::common::{
    data_class_from_openapi, data_product_source_kind_from_openapi, legal_basis_from_openapi,
};

pub(crate) fn data_product_registration_command_input_from_openapi(
    request: pds_openapi::DataProductRequest,
) -> PdsDataProductRegistrationCommandInput {
    PdsDataProductRegistrationCommandInput {
        data_product_id: request.data_product_id,
        source_kind: data_product_source_kind_from_openapi(request.source_kind),
        release_id: request.release_id,
        title: request.title,
        purpose: request.purpose,
        legal_basis: legal_basis_from_openapi(request.legal_basis),
        data_classes: request
            .data_classes
            .into_iter()
            .map(data_class_from_openapi)
            .collect(),
    }
}

pub(crate) const fn data_product_read_intent_from_openapi(
    request: pds_openapi::DataProductReadRequest,
) -> PdsDataProductReadIntent {
    PdsDataProductReadIntent {
        grant_id: request.grant_id,
        data_product_id: request.data_product_id,
    }
}

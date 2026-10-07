#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_data_product_from_row(
    (
        data_product_id,
        owner_subject,
        source_kind,
        release_id,
        title,
        purpose,
        legal_basis,
        data_classes,
        created_at,
    ): PdsDataProductRow,
) -> Result<PdsDataProductState, PdsRepositoryError> {
    let source_kind = PdsDataProductSourceKind::from_label(&source_kind)
        .ok_or(PdsRepositoryError::InvalidStoredData)?;
    let legal_basis =
        PdsLegalBasis::from_label(&legal_basis).ok_or(PdsRepositoryError::InvalidStoredData)?;
    let release_artifact = kiva_pds::release_artifact_ref_from_data_product_source_bytes(
        source_kind,
        pds_fixed_32_bytes(release_id)?,
    )
    .map_err(|_error| PdsRepositoryError::InvalidStoredData)?;
    let data_classes = pds_data_classes_from_labels(&data_classes.0)?;
    if owner_subject.is_empty() || title.is_empty() || purpose.is_empty() || data_classes.is_empty()
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsDataProductState {
        data_product_id,
        owner_subject,
        release_artifact,
        title,
        purpose,
        legal_basis,
        data_classes,
        created_at,
    })
}

pub(crate) fn pds_data_class_labels(data_classes: &[PdsDataClass]) -> Vec<String> {
    data_classes
        .iter()
        .map(|data_class| data_class.label().to_owned())
        .collect()
}

fn pds_data_classes_from_labels(
    labels: &[String],
) -> Result<Vec<PdsDataClass>, PdsRepositoryError> {
    let data_classes = labels
        .iter()
        .map(|label| PdsDataClass::from_label(label).ok_or(PdsRepositoryError::InvalidStoredData))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique_data_classes = BTreeSet::new();
    if data_classes.is_empty()
        || !data_classes
            .iter()
            .all(|data_class| unique_data_classes.insert(*data_class))
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(data_classes)
}

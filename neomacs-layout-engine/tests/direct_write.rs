#![cfg(windows)]

use dwrote::{FontCollection, FontFamily};
use neomacs_layout_engine::font_backend::{DirectWriteBackend, FontBackend, FontFamilyName};
use std::collections::HashSet;
use std::ptr;
use winapi::shared::winerror::S_OK;
use winapi::um::dwrite::IDWriteLocalizedStrings;

fn localized_family_names(family: FontFamily) -> Vec<String> {
    let mut strings: *mut IDWriteLocalizedStrings = ptr::null_mut();
    unsafe {
        if (*family.as_ptr()).GetFamilyNames(&mut strings) != S_OK || strings.is_null() {
            return Vec::new();
        }

        let mut names = Vec::new();
        for index in 0..(*strings).GetCount() {
            let mut length = 0;
            if (*strings).GetStringLength(index, &mut length) != S_OK {
                continue;
            }
            let mut buffer = vec![0; length as usize + 1];
            if (*strings).GetString(index, buffer.as_mut_ptr(), length + 1) == S_OK
                && let Ok(name) = String::from_utf16(&buffer[..length as usize])
            {
                names.push(name);
            }
        }
        (*strings).Release();
        names
    }
}

#[test]
fn list_families_includes_all_usable_directwrite_family_names() {
    let listed = DirectWriteBackend
        .list_families()
        .into_iter()
        .map(|family| family.into_string())
        .collect::<HashSet<_>>();
    let collection = FontCollection::system();
    let missing = collection
        .families_iter()
        .flat_map(localized_family_names)
        .filter_map(FontFamilyName::new)
        .filter(|family| {
            collection
                .font_family_by_name(family.as_str())
                .is_ok_and(|match_| match_.is_some())
        })
        .filter(|family| !listed.contains(family.as_str()))
        .map(|family| family.into_string())
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "DirectWrite family list omitted usable localized names: {missing:?}"
    );
}

#[test]
fn listed_families_resolve_through_directwrite() {
    let collection = FontCollection::system();
    let unresolved = DirectWriteBackend
        .list_families()
        .into_iter()
        .filter(|family| {
            collection
                .font_family_by_name(family.as_str())
                .ok()
                .flatten()
                .is_none()
        })
        .map(|family| family.into_string())
        .collect::<Vec<_>>();

    assert!(
        unresolved.is_empty(),
        "DirectWrite family list contained unresolved names: {unresolved:?}"
    );
}

#[test]
fn list_families_includes_accepted_family_alias() {
    const ALIAS: &str = "CaskaydiaCove NF";
    let collection = FontCollection::system();
    if collection
        .font_family_by_name(ALIAS)
        .ok()
        .flatten()
        .is_none()
    {
        return;
    }

    let families = DirectWriteBackend.list_families();
    assert!(
        families.iter().any(|family| family.as_str() == ALIAS),
        "DirectWrite family list omitted {ALIAS}"
    );
}

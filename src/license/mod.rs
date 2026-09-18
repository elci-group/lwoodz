// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
pub mod compatibility;
pub mod header;
pub mod spdx;
pub mod templates;

pub use compatibility::{check_compatibility, Compatibility, CompatibilityReport};
pub use header::{ensure_headers, HeaderResult};
pub use spdx::{is_valid_spdx, normalize_spdx, SpdxExpression};
pub use templates::{all_licenses, find_template, render_license, LicenseId, LicenseTemplate};

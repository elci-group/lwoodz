#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LicenseId {
    Mit,
    Apache20,
    Gpl20Only,
    Gpl30Only,
    Lgpl21Only,
    Lgpl30Only,
    Agpl30Only,
    Bsd2Clause,
    Bsd3Clause,
    Isc,
    Mpl20,
    Unlicense,
    Cc0_10,
    Bsl10,
    Zlib,
}

impl LicenseId {
    pub fn from_spdx(s: &str) -> Option<Self> {
        match crate::license::spdx::normalize_spdx(s).as_str() {
            "MIT" => Some(Self::Mit),
            "Apache-2.0" => Some(Self::Apache20),
            "GPL-2.0-only" | "GPL-2.0" => Some(Self::Gpl20Only),
            "GPL-3.0-only" | "GPL-3.0" => Some(Self::Gpl30Only),
            "LGPL-2.1-only" | "LGPL-2.1" => Some(Self::Lgpl21Only),
            "LGPL-3.0-only" | "LGPL-3.0" => Some(Self::Lgpl30Only),
            "AGPL-3.0-only" | "AGPL-3.0" => Some(Self::Agpl30Only),
            "BSD-2-Clause" => Some(Self::Bsd2Clause),
            "BSD-3-Clause" => Some(Self::Bsd3Clause),
            "ISC" => Some(Self::Isc),
            "MPL-2.0" => Some(Self::Mpl20),
            "Unlicense" => Some(Self::Unlicense),
            "CC0-1.0" => Some(Self::Cc0_10),
            "BSL-1.0" => Some(Self::Bsl10),
            "Zlib" => Some(Self::Zlib),
            _ => None,
        }
    }

    pub fn spdx_id(&self) -> &'static str {
        match self {
            Self::Mit => "MIT",
            Self::Apache20 => "Apache-2.0",
            Self::Gpl20Only => "GPL-2.0-only",
            Self::Gpl30Only => "GPL-3.0-only",
            Self::Lgpl21Only => "LGPL-2.1-only",
            Self::Lgpl30Only => "LGPL-3.0-only",
            Self::Agpl30Only => "AGPL-3.0-only",
            Self::Bsd2Clause => "BSD-2-Clause",
            Self::Bsd3Clause => "BSD-3-Clause",
            Self::Isc => "ISC",
            Self::Mpl20 => "MPL-2.0",
            Self::Unlicense => "Unlicense",
            Self::Cc0_10 => "CC0-1.0",
            Self::Bsl10 => "BSL-1.0",
            Self::Zlib => "Zlib",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Mit => "MIT License",
            Self::Apache20 => "Apache License 2.0",
            Self::Gpl20Only => "GNU General Public License v2.0 only",
            Self::Gpl30Only => "GNU General Public License v3.0 only",
            Self::Lgpl21Only => "GNU Lesser General Public License v2.1 only",
            Self::Lgpl30Only => "GNU Lesser General Public License v3.0 only",
            Self::Agpl30Only => "GNU Affero General Public License v3.0 only",
            Self::Bsd2Clause => "BSD 2-Clause \"Simplified\" License",
            Self::Bsd3Clause => "BSD 3-Clause \"New\" or \"Revised\" License",
            Self::Isc => "ISC License",
            Self::Mpl20 => "Mozilla Public License 2.0",
            Self::Unlicense => "The Unlicense",
            Self::Cc0_10 => "Creative Commons Zero v1.0 Universal",
            Self::Bsl10 => "Boost Software License 1.0",
            Self::Zlib => "zlib License",
        }
    }

    pub fn is_copyleft(&self) -> bool {
        matches!(
            self,
            Self::Gpl20Only
                | Self::Gpl30Only
                | Self::Lgpl21Only
                | Self::Lgpl30Only
                | Self::Agpl30Only
        )
    }

    pub fn is_strong_copyleft(&self) -> bool {
        matches!(self, Self::Gpl20Only | Self::Gpl30Only | Self::Agpl30Only)
    }
}

#[derive(Debug, Clone)]
pub struct LicenseTemplate {
    pub id: LicenseId,
    pub spdx: String,
    pub name: String,
    pub body: String,
}

pub fn all_licenses() -> Vec<LicenseTemplate> {
    vec![
        template_mit(),
        template_apache20(),
        template_bsd2(),
        template_bsd3(),
        template_isc(),
        template_mpl20(),
        template_gpl20(),
        template_gpl30(),
        template_lgpl21(),
        template_lgpl30(),
        template_agpl30(),
        template_unlicense(),
        template_cc0(),
        template_bsl10(),
        template_zlib(),
    ]
}

pub fn find_template(spdx: &str) -> Option<LicenseTemplate> {
    let norm = crate::license::spdx::normalize_spdx(spdx);
    all_licenses()
        .into_iter()
        .find(|t| t.spdx == norm || t.spdx == spdx)
}

pub fn render_license(spdx: &str, holder: &str, year: i32) -> anyhow::Result<String> {
    let tpl = find_template(spdx).ok_or_else(|| {
        anyhow::anyhow!("unknown license id: {} (known: {})", spdx, all_spdx_list())
    })?;
    let mut body = tpl.body.clone();
    body = body.replace("{{year}}", &year.to_string());
    body = body.replace("{{holder}}", holder);
    body = body.replace("{{spdx}}", tpl.spdx.as_str());
    Ok(body)
}

fn all_spdx_list() -> String {
    all_licenses()
        .iter()
        .map(|t| t.spdx.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Template bodies — concise but legally faithful.
// For GPL family we include the standard header + pointer to full text.
// ---------------------------------------------------------------------------

fn template_mit() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Mit,
        spdx: "MIT".to_string(),
        name: "MIT License".to_string(),
        body: r#"MIT License

Copyright (c) {{year}} {{holder}}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#
        .to_string(),
    }
}

fn template_apache20() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Apache20,
        spdx: "Apache-2.0".to_string(),
        name: "Apache License 2.0".to_string(),
        body: r#"Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/

Copyright (c) {{year}} {{holder}}

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
"#
        .to_string(),
    }
}

fn template_bsd2() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Bsd2Clause,
        spdx: "BSD-2-Clause".to_string(),
        name: "BSD 2-Clause \"Simplified\" License".to_string(),
        body: r#"BSD 2-Clause License

Copyright (c) {{year}} {{holder}}
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
"#
        .to_string(),
    }
}

fn template_bsd3() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Bsd3Clause,
        spdx: "BSD-3-Clause".to_string(),
        name: "BSD 3-Clause \"New\" or \"Revised\" License".to_string(),
        body: r#"BSD 3-Clause License

Copyright (c) {{year}} {{holder}}
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
"#
        .to_string(),
    }
}

fn template_isc() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Isc,
        spdx: "ISC".to_string(),
        name: "ISC License".to_string(),
        body: r#"ISC License

Copyright (c) {{year}} {{holder}}

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
"#
        .to_string(),
    }
}

fn template_mpl20() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Mpl20,
        spdx: "MPL-2.0".to_string(),
        name: "Mozilla Public License 2.0".to_string(),
        body: r#"Mozilla Public License Version 2.0
==================================

Copyright (c) {{year}} {{holder}}

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at http://mozilla.org/MPL/2.0/.

SPDX-License-Identifier: MPL-2.0
"#
        .to_string(),
    }
}

fn template_gpl20() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Gpl20Only,
        spdx: "GPL-2.0-only".to_string(),
        name: "GNU General Public License v2.0 only".to_string(),
        body: r#"GNU GENERAL PUBLIC LICENSE
Version 2, June 1991
Copyright (C) {{year}} {{holder}}

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 2 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License along
with this program; if not, write to the Free Software Foundation, Inc.,
51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

SPDX-License-Identifier: GPL-2.0-only
"#
        .to_string(),
    }
}

fn template_gpl30() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Gpl30Only,
        spdx: "GPL-3.0-only".to_string(),
        name: "GNU General Public License v3.0 only".to_string(),
        body: r#"GNU GENERAL PUBLIC LICENSE
Version 3, 29 June 2007
Copyright (C) {{year}} {{holder}}

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

SPDX-License-Identifier: GPL-3.0-only
"#
        .to_string(),
    }
}

fn template_lgpl21() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Lgpl21Only,
        spdx: "LGPL-2.1-only".to_string(),
        name: "GNU Lesser General Public License v2.1 only".to_string(),
        body: r#"GNU LESSER GENERAL PUBLIC LICENSE
Version 2.1, February 1999
Copyright (C) {{year}} {{holder}}

This library is free software; you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License as published by
the Free Software Foundation; either version 2.1 of the License, or
(at your option) any later version.

SPDX-License-Identifier: LGPL-2.1-only
"#
        .to_string(),
    }
}

fn template_lgpl30() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Lgpl30Only,
        spdx: "LGPL-3.0-only".to_string(),
        name: "GNU Lesser General Public License v3.0 only".to_string(),
        body: r#"GNU LESSER GENERAL PUBLIC LICENSE
Version 3, 29 June 2007
Copyright (C) {{year}} {{holder}}

This library is free software; you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License as published by
the Free Software Foundation; either version 3 of the License, or
(at your option) any later version.

SPDX-License-Identifier: LGPL-3.0-only
"#
        .to_string(),
    }
}

fn template_agpl30() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Agpl30Only,
        spdx: "AGPL-3.0-only".to_string(),
        name: "GNU Affero General Public License v3.0 only".to_string(),
        body: r#"GNU AFFERO GENERAL PUBLIC LICENSE
Version 3, 19 November 2007
Copyright (C) {{year}} {{holder}}

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published
by the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

SPDX-License-Identifier: AGPL-3.0-only
"#
        .to_string(),
    }
}

fn template_unlicense() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Unlicense,
        spdx: "Unlicense".to_string(),
        name: "The Unlicense".to_string(),
        body: r#"This is free and unencumbered software released into the public domain.

Anyone is free to copy, modify, publish, use, compile, sell, or
distribute this software, either in source code form or as a compiled
binary, for any purpose, commercial or non-commercial, and by any
means.

In jurisdictions that recognize copyright laws, the author or authors
of this software dedicate any and all copyright interest in the
software to the public domain.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.

For more information, please refer to <http://unlicense.org/>

SPDX-License-Identifier: Unlicense
"#
        .to_string(),
    }
}

fn template_cc0() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Cc0_10,
        spdx: "CC0-1.0".to_string(),
        name: "Creative Commons Zero v1.0 Universal".to_string(),
        body: r#"CC0 1.0 Universal

Statement of Purpose

The laws of most jurisdictions throughout the world automatically confer
exclusive Copyright and Related Rights upon the creator and subsequent
owner(s) of an original work. Under CC0, {{holder}} waives all copyright
and related rights to the extent permitted by law, affirming {{year}}.

SPDX-License-Identifier: CC0-1.0
"#
        .to_string(),
    }
}

fn template_bsl10() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Bsl10,
        spdx: "BSL-1.0".to_string(),
        name: "Boost Software License 1.0".to_string(),
        body: r#"Boost Software License - Version 1.0 - August 17th, 2003

Copyright (c) {{year}} {{holder}}

Permission is hereby granted, free of charge, to any person or organization
obtaining a copy of the software and accompanying documentation covered by
this license to use, reproduce, display, distribute, execute, and transmit
the Software, and to prepare derivative works of the Software, and to permit
third-parties to whom the Software is furnished to do so, all subject to the
following:

SPDX-License-Identifier: BSL-1.0
"#
        .to_string(),
    }
}

fn template_zlib() -> LicenseTemplate {
    LicenseTemplate {
        id: LicenseId::Zlib,
        spdx: "Zlib".to_string(),
        name: "zlib License".to_string(),
        body: r#"zlib License

Copyright (c) {{year}} {{holder}}

This software is provided 'as-is', without any express or implied
warranty.  In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented;
2. Altered source versions must be plainly marked as such;
3. This notice may not be removed or altered from any source distribution.

SPDX-License-Identifier: Zlib
"#
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_mit_contains_holder() {
        let out = render_license("MIT", "Acme", 2026).unwrap();
        assert!(out.contains("Acme"));
        assert!(out.contains("2026"));
    }

    #[test]
    fn unknown_license_errors() {
        assert!(render_license("WTFPL", "x", 2026).is_err());
    }
}

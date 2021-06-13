/*
 *  ******************************************************************************************
 *  Copyright (c) 2021 Pascal Kuthe. This file is part of the frontend project.
 *  It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/DSPOM/OpenVAF/blob/master/LICENSE.
 *  No part of frontend, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 *  *****************************************************************************************
 */

use crate::spec::TargetOptions;

pub fn opts() -> TargetOptions {
    TargetOptions {
        //       dll_prefix: "".to_string(),
        is_like_windows: true,
        is_like_msvc: true,
        ..Default::default()
    }
}

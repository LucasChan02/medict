//go:build !windows

// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

package desktopapi

import "os"

func replaceFile(source, target string) error {
	return os.Rename(source, target)
}

//go:build windows

// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

package desktopapi

import "golang.org/x/sys/windows"

func replaceFile(source, target string) error {
	sourcePtr, err := windows.UTF16PtrFromString(source)
	if err != nil {
		return err
	}
	targetPtr, err := windows.UTF16PtrFromString(target)
	if err != nil {
		return err
	}
	return windows.MoveFileEx(
		sourcePtr,
		targetPtr,
		windows.MOVEFILE_REPLACE_EXISTING|windows.MOVEFILE_WRITE_THROUGH,
	)
}

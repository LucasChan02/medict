// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.AboutPage {
    aboutData: {
        "displayName": "Medict",
        "productName": "Medict",
        "componentName": "medict",
        "shortDescription": qsTr("Offline dictionary for MDict and StarDict files"),
        "homepage": "https://github.com/terasum/medict",
        "bugAddress": "https://github.com/terasum/medict/issues",
        "version": appController.version,
        "otherText": qsTr("Qt %1\nDictionary engine %2")
            .arg(appController.qtVersion)
            .arg(appController.backendVersion),
        "authors": [
            {
                "name": qsTr("Medict contributors"),
                "task": qsTr("Development")
            }
        ],
        "credits": [],
        "translators": [],
        "licenses": [
            {
                "name": qsTr("GNU General Public License v3.0 or later"),
                "spdx": "GPL-3.0-or-later"
            }
        ],
        "copyrightStatement": qsTr("© 2026 Medict contributors"),
        "desktopFileName": "io.github.terasum.Medict"
    }

    getInvolvedUrl: ""
    donateUrl: ""
}

// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.AboutPage {
    id: page

    readonly property var appState: applicationWindow().state

    aboutData: {
        "displayName": "Medict",
        "productName": "Medict",
        "componentName": "medict",
        "shortDescription": qsTr("Offline dictionary for MDict and StarDict files"),
        "homepage": "https://github.com/terasum/medict",
        "bugAddress": "https://github.com/terasum/medict/issues",
        "version": appState.version,
        "otherText": qsTr("Rust dictionary engine and editor\nQt Quick / Kirigami interface"),
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

    Component.onCompleted: {
        if (Controller.smokeTest) {
            console.info("Medict smoke: about page initialized");
        }
    }

    actions: [
        BreezeAction {
            text: qsTr("Close")
            breezeIconName: "window-close"
            onTriggered: applicationWindow().pageStack.layers.pop()
        }
    ]
}

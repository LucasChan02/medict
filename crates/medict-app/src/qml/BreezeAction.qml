// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.Action {
    id: action

    property string breezeIconName: ""

    readonly property var iconAliases: ({
        "edit-clear": "edit-delete"
    })

    icon.name: breezeIconName
    icon.source: breezeIconName.length > 0
        ? "qrc:/qt/qml/medict_app/icons/breeze/scalable/actions/"
            + (iconAliases[breezeIconName] || breezeIconName) + ".svg"
        : ""
}

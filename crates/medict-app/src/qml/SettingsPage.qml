// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.ScrollablePage {
    id: page

    readonly property var appState: applicationWindow().state

    property bool allowSymlinks: true
    property int groupLimit: 10
    property bool initialized: false

    title: qsTr("Settings")

    function loadSettings(): void {
        page.allowSymlinks = appState.settings.allowSymlinks;
        page.groupLimit = appState.settings.groupLimit;
        page.initialized = true;
    }

    Component.onCompleted: page.loadSettings()

    actions: [
        BreezeAction {
            text: qsTr("Save settings")
            breezeIconName: "document-save"
            enabled: appState.ready
            onTriggered: Controller.saveSettings(page.allowSymlinks, page.groupLimit)
        },
        BreezeAction {
            text: qsTr("Close")
            breezeIconName: "window-close"
            shortcut: "Escape"
            onTriggered: applicationWindow().pageStack.layers.pop()
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            level: 2
            text: qsTr("Dictionaries")
        }

        Kirigami.FormLayout {
            Controls.TextField {
                Layout.fillWidth: true
                text: appState.baseDictDir
                readOnly: true
                Kirigami.FormData.label: qsTr("Dictionary folder:")

                Controls.ToolTip.visible: hovered
                Controls.ToolTip.text: text
            }

            Controls.Button {
                text: qsTr("Open dictionary folder")
                icon.name: "document-open-folder"
                onClicked: Controller.openDictionaryFolder()
            }

            Controls.Button {
                text: qsTr("Reload dictionaries")
                icon.name: "view-refresh"
                enabled: appState.ready && !appState.busy
                onClicked: Controller.refreshDictionaries()
            }

            Controls.CheckBox {
                text: qsTr("Load dictionaries through symbolic links")
                checked: page.allowSymlinks
                Kirigami.FormData.label: qsTr("Load rules:")
                onToggled: page.allowSymlinks = checked
            }

            Controls.SpinBox {
                from: 1
                to: 100
                value: page.groupLimit
                Kirigami.FormData.label: qsTr("Maximum dictionaries per group:")
                onValueModified: page.groupLimit = value
            }
        }

        Kirigami.Separator {
            Layout.fillWidth: true
        }

        Kirigami.Heading {
            level: 2
            text: qsTr("Full English–Chinese dictionary")
        }

        Controls.Label {
            Layout.fillWidth: true
            text: appState.ecdict.status
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.mediumSpacing

            Controls.Button {
                text: appState.ecdict.fullInstalled
                    ? qsTr("Full edition installed")
                    : qsTr("Download full ECDICT")
                icon.name: appState.ecdict.fullInstalled ? "dialog-ok" : "download"
                enabled: !appState.ecdict.busy && !appState.ecdict.fullInstalled
                onClicked: Controller.installEcdict()
            }

            Controls.ProgressBar {
                Layout.fillWidth: true
                visible: appState.ecdict.busy
                indeterminate: true
            }
        }

        Kirigami.Separator {
            Layout.fillWidth: true
        }

        Kirigami.Heading {
            level: 2
            text: qsTr("Appearance")
        }

        Controls.Label {
            Layout.fillWidth: true
            text: qsTr("Medict uses the active Plasma color scheme, the native Qt Quick "
                + "control style, and Breeze icons. The dictionary pane follows the "
                + "current light or dark system preference.")
            wrapMode: Text.WordWrap
        }

        Controls.Label {
            Layout.fillWidth: true
            text: appState.status
            wrapMode: Text.WordWrap
            opacity: 0.7
        }

        Item {
            Layout.fillHeight: true
        }
    }

    Connections {
        target: Controller

        function onStateChanged(): void {
            if (!page.initialized) {
                page.loadSettings();
            }
        }
    }
}

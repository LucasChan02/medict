// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    property bool allowSymlinks: true
    property int groupLimit: 10

    title: qsTr("Settings")

    actions: [
        Kirigami.Action {
            text: qsTr("Save settings")
            icon.name: "document-save"
            enabled: appController.ready
            onTriggered: appController.saveSettings(page.allowSymlinks, page.groupLimit)
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "window-close"
            onTriggered: applicationWindow().pageStack.layers.pop()
        }
    ]

    Component.onCompleted: {
        page.allowSymlinks = appController.allowSymlinks;
        page.groupLimit = appController.groupLimit;
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            level: 2
            text: qsTr("Dictionaries")
        }

        Kirigami.FormLayout {
            Controls.TextField {
                id: dictionaryFolder

                Layout.fillWidth: true
                text: appController.baseDictDir
                readOnly: true
                Kirigami.FormData.label: qsTr("Dictionary folder:")

                Controls.ToolTip.visible: hovered
                Controls.ToolTip.text: text
            }

            Controls.Button {
                text: qsTr("Open dictionary folder")
                icon.name: "document-open-folder"
                onClicked: appController.openDictionaryFolder()
            }

            Controls.Button {
                text: qsTr("Reload dictionaries")
                icon.name: "view-refresh"
                enabled: appController.ready
                onClicked: appController.refreshDictionaries()
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
            text: appController.ecdictStatus
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.mediumSpacing

            Controls.Button {
                text: appController.ecdictFullInstalled
                    ? qsTr("Full edition installed")
                    : qsTr("Download full ECDICT")
                icon.name: appController.ecdictFullInstalled ? "dialog-ok" : "download"
                enabled: !appController.ecdictBusy && !appController.ecdictFullInstalled
                onClicked: appController.installEcdict()
            }

            Controls.ProgressBar {
                Layout.fillWidth: true
                visible: appController.ecdictBusy
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
            text: qsTr("Medict uses the active Plasma color scheme, control style, "
                + "and Breeze icons when they are available. Other desktops use "
                + "the native Qt Quick style and the embedded Breeze icons.")
            wrapMode: Text.WordWrap
        }

        Controls.Label {
            Layout.fillWidth: true
            text: appController.statusText
            wrapMode: Text.WordWrap
            opacity: 0.7
        }

        Item {
            Layout.fillHeight: true
        }
    }

    Connections {
        target: appController

        function onSettingsChanged(): void {
            page.allowSymlinks = appController.allowSymlinks;
            page.groupLimit = appController.groupLimit;
        }
    }
}

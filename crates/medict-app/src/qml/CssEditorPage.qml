// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.Page {
    id: page

    readonly property var appState: applicationWindow().state

    property bool dirty: false
    property bool loadingText: false

    title: appState.css.dictName.length > 0
        ? qsTr("CSS — %1").arg(appState.css.dictName)
        : qsTr("Dictionary CSS")

    function loadEditorText(): void {
        page.loadingText = true;
        cssEditor.text = appState.css.text;
        page.loadingText = false;
        page.dirty = false;
    }

    function closeEditor(): void {
        applicationWindow().previewCss = "";
        if (applicationWindow().pageStack.layers.depth > 1) {
            applicationWindow().pageStack.layers.pop();
        }
        applicationWindow().reloadDefinitions();
    }

    Component.onCompleted: {
        page.loadEditorText();
        cssEditor.forceActiveFocus();
    }

    Component.onDestruction: {
        applicationWindow().previewCss = "";
        applicationWindow().reloadDefinitions();
    }

    actions: [
        BreezeAction {
            text: qsTr("Save CSS")
            breezeIconName: "document-save"
            shortcut: "Ctrl+S"
            enabled: !appState.css.busy && appState.css.dictId.length > 0
            onTriggered: {
                page.dirty = false;
                Controller.saveCss(cssEditor.text);
            }
        },
        BreezeAction {
            text: qsTr("Restore dictionary CSS")
            breezeIconName: "view-refresh"
            enabled: !appState.css.busy
            onTriggered: {
                Controller.restoreBaseCss();
                Qt.callLater(page.loadEditorText);
            }
        },
        BreezeAction {
            text: qsTr("Remove override")
            breezeIconName: "edit-delete"
            enabled: !appState.css.busy && appState.css.dictId.length > 0
            onTriggered: removeCssDialog.open()
        },
        BreezeAction {
            text: qsTr("Close")
            breezeIconName: "window-close"
            shortcut: "Escape"
            onTriggered: page.closeEditor()
        }
    ]

    ColumnLayout {
        anchors.fill: parent
        spacing: Kirigami.Units.smallSpacing

        Controls.Label {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            Layout.bottomMargin: 0
            text: qsTr("This override applies only to %1. The preview updates while you type.")
                .arg(appState.css.dictName)
            wrapMode: Text.WordWrap
            opacity: 0.8
        }

        Controls.ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            clip: true

            Controls.TextArea {
                id: cssEditor

                wrapMode: TextEdit.NoWrap
                selectByMouse: true
                font: Kirigami.Theme.fixedWidthFont
                placeholderText: qsTr("Enter dictionary CSS")

                onTextChanged: {
                    if (!page.loadingText) {
                        page.dirty = true;
                        previewTimer.restart();
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            Layout.topMargin: 0
            spacing: Kirigami.Units.smallSpacing

            Controls.BusyIndicator {
                visible: appState.css.busy
                running: visible
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }

            Controls.Label {
                Layout.fillWidth: true
                text: appState.css.status
                wrapMode: Text.WordWrap
            }
        }
    }

    Timer {
        id: previewTimer
        interval: 300
        onTriggered: applicationWindow().previewCss = cssEditor.text
    }

    Kirigami.PromptDialog {
        id: removeCssDialog

        title: qsTr("Remove CSS override")
        subtitle: qsTr("The dictionary shows its original CSS after removal.")
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: {
            page.dirty = false;
            Controller.removeCss();
        }
    }

    Connections {
        target: Controller

        function onStateChanged(): void {
            if (!page.dirty) {
                page.loadEditorText();
            }
        }
    }
}

// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: page

    property bool loadingCss: true

    title: appController.cssDictionaryName.length > 0
        ? qsTr("CSS — %1").arg(appController.cssDictionaryName)
        : qsTr("Dictionary CSS")

    function loadEditorText(): void {
        page.loadingCss = true;
        cssEditor.text = appController.cssText;
        page.loadingCss = false;
    }

    function closeEditor(): void {
        if (applicationWindow().pageStack.layers.depth > 0) {
            applicationWindow().pageStack.layers.pop();
        }
    }

    Component.onCompleted: {
        page.loadEditorText();
        cssEditor.forceActiveFocus();
    }

    Component.onDestruction: appController.finishCssEditor()

    actions: [
        Kirigami.Action {
            text: qsTr("Save CSS")
            icon.name: "document-save"
            shortcut: "Ctrl+S"
            enabled: !appController.cssBusy
            onTriggered: appController.saveCss(cssEditor.text)
        },
        Kirigami.Action {
            text: qsTr("Restore dictionary CSS")
            icon.name: "view-refresh"
            enabled: !appController.cssBusy
            onTriggered: {
                appController.restoreBaseCss();
                page.loadEditorText();
            }
        },
        Kirigami.Action {
            text: qsTr("Remove override")
            icon.name: "edit-delete"
            enabled: !appController.cssBusy
            onTriggered: removeCssDialog.open()
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "window-close"
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
                .arg(appController.cssDictionaryName)
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
                    if (!page.loadingCss) {
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
                visible: appController.cssBusy
                running: visible
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }

            Controls.Label {
                Layout.fillWidth: true
                text: appController.cssStatus
                wrapMode: Text.WordWrap
            }
        }
    }

    Timer {
        id: previewTimer
        interval: 300
        onTriggered: appController.previewCss(cssEditor.text)
    }

    Kirigami.PromptDialog {
        id: removeCssDialog

        title: qsTr("Remove CSS override")
        subtitle: qsTr("The dictionary shows its original CSS after removal.")
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: {
            appController.removeCssOverride();
            page.loadEditorText();
        }
    }

    Connections {
        target: appController

        function onCssTextChanged(): void {
            page.loadEditorText();
        }
    }
}

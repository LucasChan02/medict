// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.Page {
    id: page

    readonly property var appState: applicationWindow().state
    readonly property bool wide: width >= Kirigami.Units.gridUnit * 50
    readonly property bool hasNotebook: appState.selectedNotebookId.length > 0
    readonly property var notebookModel: [{
        "id": "",
        "name": qsTr("All words"),
        "displayName": qsTr("All words"),
        "isDefault": false
    }].concat(appState.notebooks || [])
    readonly property var visibleBookmarks: {
        const notebookId = appState.selectedNotebookId;
        const all = appState.bookmarks || [];
        return notebookId.length === 0
            ? all
            : all.filter(bookmark => bookmark.notebookId === notebookId);
    }

    property var removeTarget: null

    title: qsTr("Vocabulary notebooks")
    padding: 0

    function currentNotebook(): var {
        return (appState.notebooks || []).find(
            notebook => notebook.id === appState.selectedNotebookId) || null;
    }

    function openBookmark(bookmark): void {
        if (!bookmark || bookmark.word.length === 0) {
            return;
        }
        if (applicationWindow().pageStack.layers.depth > 1) {
            applicationWindow().pageStack.layers.pop();
        }
        Controller.activateBookmark(bookmark.word, bookmark.dictId);
    }

    function openCreateDialog(): void {
        notebookNameDialog.renaming = false;
        notebookNameDialog.open();
    }

    function openRenameDialog(): void {
        if (!page.currentNotebook()) {
            return;
        }
        notebookNameDialog.renaming = true;
        notebookNameDialog.open();
    }

    function submitNotebookName(): void {
        const name = notebookNameField.text.trim();
        if (name.length === 0) {
            return;
        }
        if (notebookNameDialog.renaming) {
            Controller.renameNotebook(appState.selectedNotebookId, name);
        } else {
            Controller.createNotebook(name);
        }
        notebookNameDialog.close();
    }

    function syncNotebookCombo(): void {
        const rows = page.notebookModel;
        for (let index = 0; index < rows.length; ++index) {
            if ((rows[index].id || "") === appState.selectedNotebookId) {
                narrowNotebookCombo.currentIndex = index;
                return;
            }
        }
        narrowNotebookCombo.currentIndex = 0;
    }

    actions: [
        BreezeAction {
            text: qsTr("New notebook")
            breezeIconName: "list-add"
            shortcut: "Ctrl+Shift+N"
            onTriggered: page.openCreateDialog()
        },
        BreezeAction {
            text: qsTr("Rename notebook")
            breezeIconName: "edit-rename"
            enabled: page.hasNotebook
            onTriggered: page.openRenameDialog()
        },
        BreezeAction {
            text: qsTr("Delete notebook")
            breezeIconName: "edit-delete"
            enabled: page.hasNotebook
            onTriggered: {
                deleteNotebookDialog.notebookId = appState.selectedNotebookId;
                deleteNotebookDialog.open();
            }
        },
        BreezeAction {
            text: qsTr("Set as default")
            breezeIconName: "dialog-ok"
            enabled: page.hasNotebook
                && page.currentNotebook()
                && !page.currentNotebook().isDefault
            onTriggered: Controller.setDefaultNotebook(appState.selectedNotebookId)
        },
        BreezeAction {
            text: qsTr("Export Anki package")
            breezeIconName: "document-export"
            enabled: page.visibleBookmarks.length > 0
            onTriggered: exportAnkiDialog.open()
        },
        BreezeAction {
            text: qsTr("Refresh")
            breezeIconName: "view-refresh"
            enabled: !appState.notebooksBusy
            onTriggered: Controller.loadNotebooks()
        },
        BreezeAction {
            text: qsTr("Close")
            breezeIconName: "window-close"
            shortcut: "Escape"
            onTriggered: applicationWindow().pageStack.layers.pop()
        }
    ]

    footer: Controls.Pane {
        visible: !Kirigami.Settings.isMobile
        padding: Kirigami.Units.smallSpacing

        RowLayout {
            anchors.fill: parent
            spacing: Kirigami.Units.smallSpacing

            Controls.BusyIndicator {
                visible: appState.notebooksBusy
                running: visible
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }

            Controls.Label {
                Layout.fillWidth: true
                text: appState.notebooksBusy
                    ? qsTr("Loading saved words…")
                    : qsTr("%1 saved word(s)").arg(page.visibleBookmarks.length)
                elide: Text.ElideRight
            }

            Controls.Label {
                text: appState.status
                opacity: 0.7
                elide: Text.ElideRight
                Layout.maximumWidth: page.width / 2
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Controls.Pane {
            visible: !page.wide
            Layout.fillWidth: true
            padding: Kirigami.Units.smallSpacing

            RowLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.smallSpacing

                Controls.Label {
                    text: qsTr("Notebook")
                }

                Controls.ComboBox {
                    id: narrowNotebookCombo

                    Layout.fillWidth: true
                    textRole: "displayName"
                    valueRole: "id"
                    model: page.notebookModel
                    onActivated: Controller.selectNotebook(currentValue)
                }

                Controls.ToolButton {
                    text: qsTr("Notebook actions")
                    icon.name: "overflow-menu"
                    onClicked: notebookMenu.open()
                    Controls.ToolTip.visible: hovered
                    Controls.ToolTip.text: text

                    Controls.Menu {
                        id: notebookMenu

                        Controls.MenuItem {
                            text: qsTr("New notebook")
                            icon.name: "list-add"
                            onTriggered: page.openCreateDialog()
                        }
                        Controls.MenuItem {
                            text: qsTr("Rename notebook")
                            icon.name: "edit-rename"
                            enabled: page.hasNotebook
                            onTriggered: page.openRenameDialog()
                        }
                        Controls.MenuItem {
                            text: qsTr("Delete notebook")
                            icon.name: "edit-delete"
                            enabled: page.hasNotebook
                            onTriggered: {
                                deleteNotebookDialog.notebookId = appState.selectedNotebookId;
                                deleteNotebookDialog.open();
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Controls.Frame {
                visible: page.wide
                Layout.preferredWidth: Kirigami.Units.gridUnit * 18
                Layout.fillHeight: true
                padding: Kirigami.Units.smallSpacing

                ColumnLayout {
                    anchors.fill: parent
                    spacing: Kirigami.Units.smallSpacing

                    Controls.Label {
                        Layout.leftMargin: Kirigami.Units.smallSpacing
                        text: qsTr("Notebooks")
                        font.weight: Font.DemiBold
                    }

                    ListView {
                        id: notebookList

                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: page.notebookModel

                        delegate: Controls.ItemDelegate {
                            required property var modelData

                            width: notebookList.width
                            text: modelData.displayName
                            icon.name: modelData.isDefault ? "dialog-ok" : "bookmarks"
                            highlighted: appState.selectedNotebookId === modelData.id
                            onClicked: Controller.selectNotebook(modelData.id)
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        Controls.ToolButton {
                            text: qsTr("New notebook")
                            icon.name: "list-add"
                            onClicked: page.openCreateDialog()
                            Controls.ToolTip.visible: hovered
                            Controls.ToolTip.text: text
                        }

                        Controls.ToolButton {
                            text: qsTr("Rename notebook")
                            icon.name: "edit-rename"
                            enabled: page.hasNotebook
                            onClicked: page.openRenameDialog()
                            Controls.ToolTip.visible: hovered
                            Controls.ToolTip.text: text
                        }

                        Controls.ToolButton {
                            text: qsTr("Delete notebook")
                            icon.name: "edit-delete"
                            enabled: page.hasNotebook
                            onClicked: {
                                deleteNotebookDialog.notebookId = appState.selectedNotebookId;
                                deleteNotebookDialog.open();
                            }
                            Controls.ToolTip.visible: hovered
                            Controls.ToolTip.text: text
                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        Controls.ToolButton {
                            text: qsTr("Set as default")
                            icon.name: "dialog-ok"
                            enabled: page.hasNotebook
                                && page.currentNotebook()
                                && !page.currentNotebook().isDefault
                            onClicked: Controller.setDefaultNotebook(appState.selectedNotebookId)
                            Controls.ToolTip.visible: hovered
                            Controls.ToolTip.text: text
                        }
                    }
                }
            }

            Controls.Frame {
                Layout.fillWidth: true
                Layout.fillHeight: true
                padding: 0

                ListView {
                    id: bookmarkList

                    anchors.fill: parent
                    clip: true
                    model: page.visibleBookmarks
                    reuseItems: true

                    delegate: Controls.ItemDelegate {
                        id: bookmarkDelegate

                        required property var modelData

                        width: bookmarkList.width
                        onClicked: page.openBookmark(modelData)

                        contentItem: RowLayout {
                            spacing: Kirigami.Units.smallSpacing

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0

                                Controls.Label {
                                    Layout.fillWidth: true
                                    text: bookmarkDelegate.modelData.word
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }

                                Controls.Label {
                                    Layout.fillWidth: true
                                    text: qsTr("%1 · %2")
                                        .arg(bookmarkDelegate.modelData.dictName)
                                        .arg(bookmarkDelegate.modelData.savedAtText)
                                    opacity: 0.7
                                    font: Kirigami.Theme.smallFont
                                    elide: Text.ElideRight
                                }
                            }

                            Controls.ToolButton {
                                text: qsTr("Remove saved word")
                                icon.name: "edit-delete"
                                onClicked: {
                                    page.removeTarget = bookmarkDelegate.modelData;
                                    removeBookmarkDialog.open();
                                }
                                Controls.ToolTip.visible: hovered
                                Controls.ToolTip.text: text
                            }
                        }
                    }
                }

                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 4
                    visible: page.visibleBookmarks.length === 0 && !appState.notebooksBusy
                    icon.name: "bookmarks"
                    text: qsTr("No saved words")
                    explanation: qsTr("Open a definition and select Save word to add it here.")
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: notebookNameDialog

        property bool renaming: false

        title: renaming ? qsTr("Rename notebook") : qsTr("New notebook")
        subtitle: renaming
            ? qsTr("Enter a new name for this notebook.")
            : qsTr("Enter a name for the new notebook.")
        standardButtons: Kirigami.Dialog.NoButton
        customFooterActions: [
            BreezeAction {
                text: notebookNameDialog.renaming ? qsTr("Rename") : qsTr("Create")
                breezeIconName: "document-save"
                enabled: notebookNameField.text.trim().length > 0
                onTriggered: page.submitNotebookName()
            },
            BreezeAction {
                text: qsTr("Cancel")
                breezeIconName: "dialog-cancel"
                onTriggered: notebookNameDialog.close()
            }
        ]
        onOpened: {
            const notebook = page.currentNotebook();
            notebookNameField.text = renaming && notebook ? notebook.name : "";
            notebookNameField.forceActiveFocus();
            notebookNameField.selectAll();
        }

        ColumnLayout {
            Controls.TextField {
                id: notebookNameField

                Layout.fillWidth: true
                placeholderText: qsTr("Notebook name")
                onAccepted: {
                    if (text.trim().length > 0) {
                        page.submitNotebookName();
                    }
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: deleteNotebookDialog

        property string notebookId

        title: qsTr("Delete notebook")
        subtitle: qsTr("The saved words move to the default notebook before deletion.")
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: Controller.deleteNotebook(notebookId)
    }

    Kirigami.PromptDialog {
        id: removeBookmarkDialog

        title: qsTr("Remove saved word")
        subtitle: page.removeTarget
            ? qsTr("Remove “%1” from this notebook?").arg(page.removeTarget.word)
            : ""
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: {
            if (page.removeTarget) {
                Controller.removeBookmark(
                    page.removeTarget.word,
                    page.removeTarget.dictId,
                    page.removeTarget.notebookId);
                page.removeTarget = null;
            }
        }
        onRejected: page.removeTarget = null
    }

    FileDialog {
        id: exportAnkiDialog

        title: qsTr("Export Anki package")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "apkg"
        nameFilters: [qsTr("Anki packages (*.apkg)")]
        onAccepted: Controller.exportAnki(
            Controller.localFile(selectedFile.toString()),
            appState.selectedNotebookId)
    }

    Connections {
        target: Controller

        function onStateChanged(): void {
            page.syncNotebookCombo();
        }
    }
}

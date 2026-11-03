// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: page

    readonly property bool wide: width >= Kirigami.Units.gridUnit * 44
    readonly property bool hasNotebook: appController.selectedNotebookId.length > 0
    readonly property var notebookModel: {
        const rows = [{
            "id": "",
            "name": qsTr("All words"),
            "displayName": qsTr("All words"),
            "isDefault": false
        }];
        return rows.concat(appController.notebooks);
    }
    readonly property var visibleBookmarks: {
        const notebookId = appController.selectedNotebookId;
        if (notebookId.length === 0) {
            return appController.bookmarks;
        }
        return appController.bookmarks.filter(bookmark => bookmark.notebookId === notebookId);
    }

    property var removeTarget: null

    title: qsTr("Vocabulary notebooks")

    function currentNotebook(): var {
        const notebookId = appController.selectedNotebookId;
        return appController.notebooks.find(notebook => notebook.id === notebookId) || null;
    }

    function openBookmark(bookmark): void {
        if (!bookmark || bookmark.word.length === 0) {
            return;
        }
        if (applicationWindow().pageStack.layers.depth > 0) {
            applicationWindow().pageStack.layers.pop();
        }
        appController.activateBookmark(bookmark.word, bookmark.dictId);
    }

    function openCreateDialog(): void {
        notebookNameDialog.renaming = false;
        notebookNameDialog.open();
    }

    function openRenameDialog(): void {
        const notebook = page.currentNotebook();
        if (!notebook) {
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
            appController.renameNotebook(appController.selectedNotebookId, name);
        } else {
            appController.createNotebook(name);
        }
        notebookNameDialog.close();
    }

    function confirmDeleteNotebook(): void {
        if (!page.hasNotebook) {
            return;
        }
        deleteNotebookDialog.notebookId = appController.selectedNotebookId;
        deleteNotebookDialog.open();
    }

    function confirmRemoveBookmark(bookmark): void {
        if (!bookmark) {
            return;
        }
        page.removeTarget = bookmark;
        removeBookmarkDialog.open();
    }

    function syncNotebookCombo(): void {
        const rows = page.notebookModel;
        for (let index = 0; index < rows.length; ++index) {
            if (rows[index].id === appController.selectedNotebookId) {
                narrowNotebookCombo.currentIndex = index;
                return;
            }
        }
        narrowNotebookCombo.currentIndex = 0;
    }

    actions: [
        Kirigami.Action {
            text: qsTr("New notebook")
            icon.name: "list-add"
            shortcut: "Ctrl+Shift+N"
            onTriggered: page.openCreateDialog()
        },
        Kirigami.Action {
            text: qsTr("Rename notebook")
            icon.name: "edit-rename"
            enabled: page.hasNotebook
            onTriggered: page.openRenameDialog()
        },
        Kirigami.Action {
            text: qsTr("Delete notebook")
            icon.name: "edit-delete"
            enabled: page.hasNotebook
            onTriggered: page.confirmDeleteNotebook()
        },
        Kirigami.Action {
            text: qsTr("Set as default")
            icon.name: "dialog-ok"
            enabled: page.hasNotebook
                && page.currentNotebook()
                && !page.currentNotebook().isDefault
            onTriggered: appController.setDefaultNotebook(appController.selectedNotebookId)
        },
        Kirigami.Action {
            text: qsTr("Export Anki package")
            icon.name: "document-export"
            enabled: page.visibleBookmarks.length > 0
            onTriggered: exportAnkiDialog.open()
        },
        Kirigami.Action {
            text: qsTr("Refresh")
            icon.name: "view-refresh"
            enabled: !appController.notebooksBusy
            onTriggered: appController.loadNotebooks()
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "window-close"
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
                visible: appController.notebooksBusy
                running: visible
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }

            Controls.Label {
                Layout.fillWidth: true
                text: appController.notebooksBusy
                    ? qsTr("Loading saved words…")
                    : qsTr("%1 saved word(s)").arg(page.visibleBookmarks.length)
                elide: Text.ElideRight
            }

            Controls.Label {
                text: appController.statusText
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
                    onActivated: appController.selectNotebook(currentValue)
                }

                Controls.ToolButton {
                    id: notebookMenuButton

                    text: qsTr("Notebook actions")
                    icon.name: "overflow-menu"
                    onClicked: notebookMenu.open()

                    Controls.ToolTip.visible: hovered
                    Controls.ToolTip.text: text

                    Controls.Menu {
                        id: notebookMenu
                        parent: notebookMenuButton
                        y: notebookMenuButton.height

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
                            onTriggered: page.confirmDeleteNotebook()
                        }
                        Controls.MenuSeparator {}
                        Controls.MenuItem {
                            text: qsTr("Set as default")
                            icon.name: "dialog-ok"
                            enabled: page.hasNotebook
                                && page.currentNotebook()
                                && !page.currentNotebook().isDefault
                            onTriggered: appController.setDefaultNotebook(
                                appController.selectedNotebookId)
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
                Layout.preferredWidth: Kirigami.Units.gridUnit * 17
                Layout.fillHeight: true
                padding: Kirigami.Units.smallSpacing

                ColumnLayout {
                    anchors.fill: parent
                    spacing: Kirigami.Units.smallSpacing

                    Controls.Label {
                        text: qsTr("Notebooks")
                        font.weight: Font.DemiBold
                        Layout.leftMargin: Kirigami.Units.smallSpacing
                    }

                    ListView {
                        id: notebookList

                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: page.notebookModel
                        currentIndex: -1

                        delegate: Controls.ItemDelegate {
                            id: notebookDelegate

                            required property var modelData

                            width: notebookList.width
                            text: modelData.displayName
                            icon.name: modelData.isDefault ? "dialog-ok" : "bookmarks"
                            highlighted: appController.selectedNotebookId === modelData.id
                            onClicked: appController.selectNotebook(modelData.id)
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
                            onClicked: page.confirmDeleteNotebook()

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
                            onClicked: appController.setDefaultNotebook(
                                appController.selectedNotebookId)

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
                                onClicked: page.confirmRemoveBookmark(
                                    bookmarkDelegate.modelData)

                                Controls.ToolTip.visible: hovered
                                Controls.ToolTip.text: text
                            }
                        }
                    }
                }

                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 4
                    visible: page.visibleBookmarks.length === 0
                        && !appController.notebooksBusy
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
            Kirigami.Action {
                text: notebookNameDialog.renaming ? qsTr("Rename") : qsTr("Create")
                icon.name: "document-save"
                enabled: notebookNameField.text.trim().length > 0
                onTriggered: page.submitNotebookName()
            },
            Kirigami.Action {
                text: qsTr("Cancel")
                icon.name: "dialog-cancel"
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
        onAccepted: appController.deleteNotebook(notebookId)
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
                appController.removeBookmark(
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
        onAccepted: appController.exportAnki(
            appController.localFile(selectedFile),
            appController.selectedNotebookId)
    }

    Connections {
        target: appController

        function onSelectedNotebookChanged(): void {
            page.syncNotebookCombo();
        }
    }
}

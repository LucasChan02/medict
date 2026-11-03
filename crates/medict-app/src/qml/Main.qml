// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.ApplicationWindow {
    id: root

    readonly property bool narrow: width < Kirigami.Units.gridUnit * 48
    readonly property var state: {
        try {
            return JSON.parse(Controller.stateJson);
        } catch (error) {
            return {
                "version": "",
                "ready": false,
                "busy": false,
                "status": qsTr("Starting…"),
                "error": "",
                "baseUrl": "",
                "dictionaries": [],
                "selectedDictIds": [],
                "currentDictId": "",
                "multiMode": false,
                "suggestions": [],
                "matches": [],
                "currentWord": "",
                "definitions": [],
                "zoom": 100,
                "canGoBack": false,
                "canGoForward": false,
                "notebooks": [],
                "bookmarks": [],
                "selectedNotebookId": "",
                "notebooksBusy": false,
                "ecdict": { "status": "", "busy": false, "fullInstalled": false },
                "settings": { "allowSymlinks": true, "groupLimit": 10 },
                "css": {
                    "dictId": "",
                    "dictName": "",
                    "text": "",
                    "baseText": "",
                    "status": "",
                    "busy": false
                }
            };
        }
    }

    property string previewCss: ""

    title: state.currentWord.length > 0
        ? qsTr("%1 — Medict").arg(state.currentWord)
        : qsTr("Medict")
    width: Kirigami.Units.gridUnit * 78
    height: Kirigami.Units.gridUnit * 52
    minimumWidth: Kirigami.Units.gridUnit * 26
    minimumHeight: Kirigami.Units.gridUnit * 24
    visible: true

    Component.onCompleted: Controller.start()

    function closeLayers(): void {
        while (pageStack.layers.depth > 0) {
            pageStack.layers.pop();
        }
    }

    function focusLookup(): void {
        closeLayers();
        if (pageStack.currentItem && pageStack.currentItem.focusSearch) {
            pageStack.currentItem.focusSearch();
        }
    }

    function openSettings(): void {
        closeLayers();
        pageStack.layers.push(settingsComponent);
    }

    function openNotebooks(): void {
        closeLayers();
        Controller.loadNotebooks();
        pageStack.layers.push(notebooksComponent);
    }

    function openCssEditor(): void {
        closeLayers();
        previewCss = "";
        Controller.prepareCssEditor();
        pageStack.layers.push(cssEditorComponent);
    }

    function openAbout(): void {
        closeLayers();
        pageStack.layers.push(aboutComponent);
    }

    function reloadDefinitions(): void {
        if (pageStack.currentItem && pageStack.currentItem.reloadDefinitions) {
            pageStack.currentItem.reloadDefinitions();
        }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        id: drawer

        modal: root.narrow
        collapsible: !root.narrow
        width: Math.min(root.width * 0.84, Kirigami.Units.gridUnit * 22)
        title: qsTr("Medict")
        titleIcon: "qrc:/qt/qml/medict_app/icons/medict.png"

        actions: [
            Kirigami.Action {
                text: qsTr("Search")
                icon.name: "edit-find"
                shortcut: "Ctrl+F"
                onTriggered: {
                    drawer.close();
                    root.focusLookup();
                }
            },
            Kirigami.Action {
                text: qsTr("Vocabulary notebooks")
                icon.name: "bookmarks"
                shortcut: "Ctrl+Shift+B"
                onTriggered: {
                    drawer.close();
                    root.openNotebooks();
                }
            },
            Kirigami.Action {
                text: qsTr("Settings")
                icon.name: "configure"
                shortcut: "Ctrl+,"
                onTriggered: {
                    drawer.close();
                    root.openSettings();
                }
            },
            Kirigami.Action {
                text: qsTr("About Medict")
                icon.name: "help-about"
                onTriggered: {
                    drawer.close();
                    root.openAbout();
                }
            }
        ]

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Controls.Label {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.mediumSpacing
                text: qsTr("Dictionaries")
                font.weight: Font.DemiBold
            }

            Controls.TextField {
                id: dictionaryFilter

                Layout.fillWidth: true
                placeholderText: qsTr("Filter dictionaries")
                rightPadding: dictionaryFilter.text.length > 0
                    ? clearDictionaryFilter.implicitWidth
                        + Kirigami.Units.smallSpacing * 2
                    : Kirigami.Units.smallSpacing * 2

                Controls.ToolButton {
                    id: clearDictionaryFilter

                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    visible: dictionaryFilter.text.length > 0
                    display: Controls.AbstractButton.IconOnly
                    icon.name: "edit-clear"
                    text: qsTr("Clear filter")
                    Accessible.name: text
                    Controls.ToolTip.visible: hovered
                    Controls.ToolTip.text: text
                    onClicked: {
                        dictionaryFilter.clear();
                        dictionaryFilter.forceActiveFocus();
                    }
                }
            }

            Repeater {
                model: (root.state.dictionaries || []).filter(dictionary => {
                    const needle = dictionaryFilter.text.trim().toLowerCase();
                    return needle.length === 0
                        || dictionary.name.toLowerCase().includes(needle);
                })

                delegate: Controls.ItemDelegate {
                    id: dictionaryDelegate

                    required property var modelData

                    Layout.fillWidth: true
                    text: modelData.name
                    highlighted: modelData.current
                    checkable: root.state.multiMode
                    checked: modelData.selected
                    Accessible.description: modelData.description.length > 0
                        ? modelData.description
                        : modelData.directory

                    Controls.ToolTip.visible: hovered
                    Controls.ToolTip.text: modelData.description.length > 0
                        ? modelData.description + "\n" + modelData.directory
                        : modelData.directory
                    onClicked: Controller.selectDictionary(modelData.id)
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: (root.state.dictionaries || []).length === 0
                text: qsTr("No dictionaries found.")
                opacity: 0.7
                wrapMode: Text.WordWrap
            }

            Controls.CheckBox {
                Layout.topMargin: Kirigami.Units.smallSpacing
                text: qsTr("Look up in several dictionaries")
                checked: root.state.multiMode
                onToggled: Controller.setMultiMode(checked)
            }

            Kirigami.Separator {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                Layout.bottomMargin: Kirigami.Units.smallSpacing
            }

            Controls.Label {
                Layout.fillWidth: true
                text: qsTr("Matches")
                font.weight: Font.DemiBold
            }

            Repeater {
                model: root.state.matches || []

                delegate: Controls.ItemDelegate {
                    required property var modelData

                    Layout.fillWidth: true
                    text: modelData
                    onClicked: {
                        drawer.close();
                        Controller.lookup(modelData);
                    }
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: (root.state.matches || []).length === 0
                text: qsTr("Start typing to see matches.")
                opacity: 0.7
                wrapMode: Text.WordWrap
            }

            Controls.Button {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                text: qsTr("Refresh dictionaries")
                icon.name: "view-refresh"
                enabled: root.state.ready && !root.state.busy
                onClicked: Controller.refreshDictionaries()
            }
        }
    }

    pageStack.initialPage: LookupPage {}

    Component {
        id: settingsComponent
        SettingsPage {}
    }

    Component {
        id: notebooksComponent
        NotebooksPage {}
    }

    Component {
        id: cssEditorComponent
        CssEditorPage {}
    }

    Component {
        id: aboutComponent
        AboutPage {}
    }

    Timer {
        running: Controller.smokeTest
        interval: 1800
        repeat: false
        onTriggered: Qt.quit()
    }
}

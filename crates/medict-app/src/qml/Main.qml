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
    property int smokeStep: 0
    property int smokeTicks: 0
    property bool smokeHasLookup: false

    title: state.currentWord.length > 0
        ? qsTr("%1 — Medict").arg(state.currentWord)
        : qsTr("Medict")
    width: Kirigami.Units.gridUnit * 78
    height: Kirigami.Units.gridUnit * 52
    minimumWidth: Kirigami.Units.gridUnit * 26
    minimumHeight: Kirigami.Units.gridUnit * 24
    visible: true

    Component.onCompleted: {
        Controller.markUiReady();
        Controller.start();
    }

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
        titleIcon: "qrc:/qt/qml/icons/medict.png"

        actions: [
            BreezeAction {
                text: qsTr("Search")
                breezeIconName: "edit-find"
                shortcut: "Ctrl+F"
                onTriggered: {
                    drawer.close();
                    root.focusLookup();
                }
            },
            BreezeAction {
                text: qsTr("Vocabulary notebooks")
                breezeIconName: "bookmarks"
                shortcut: "Ctrl+Shift+B"
                onTriggered: {
                    drawer.close();
                    root.openNotebooks();
                }
            },
            BreezeAction {
                text: qsTr("Settings")
                breezeIconName: "configure"
                shortcut: "Ctrl+,"
                onTriggered: {
                    drawer.close();
                    root.openSettings();
                }
            },
            BreezeAction {
                text: qsTr("About Medict")
                breezeIconName: "help-about"
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
        id: smokeTimer

        running: Controller.smokeTest
        interval: 250
        repeat: true
        onTriggered: {
            root.smokeTicks += 1;
            if (root.smokeTicks > 240) {
                console.error("Medict smoke test timed out at step", root.smokeStep);
                smokeTimer.stop();
                Qt.quit();
                return;
            }

            if (root.smokeStep === 0) {
                if (!root.state.ready || root.state.busy) {
                    return;
                }
                if ((root.state.dictionaries || []).length === 0) {
                    console.error("Medict smoke test found no dictionaries");
                    smokeTimer.stop();
                    Qt.quit();
                    return;
                }
                Controller.lookup("part");
                root.smokeStep = 1;
            } else if (root.smokeStep === 1) {
                if (root.state.busy || (root.state.definitions || []).length === 0) {
                    return;
                }
                const lookupPage = pageStack.currentItem;
                if (!lookupPage || !lookupPage.webEngineLoaded) {
                    return;
                }
                root.smokeHasLookup = true;
                Controller.addBookmark();
                root.smokeStep = 2;
            } else if (root.smokeStep === 2) {
                if (!root.smokeHasLookup
                        || root.state.notebooksBusy
                        || (root.state.bookmarks || []).length === 0) {
                    return;
                }
                root.openSettings();
                root.smokeStep = 3;
            } else if (root.smokeStep === 3) {
                if (pageStack.layers.depth !== 1) {
                    return;
                }
                root.openNotebooks();
                root.smokeStep = 4;
            } else if (root.smokeStep === 4) {
                if (pageStack.layers.depth !== 1) {
                    return;
                }
                root.openCssEditor();
                root.smokeStep = 5;
            } else if (root.smokeStep === 5) {
                if (pageStack.layers.depth !== 1) {
                    return;
                }
                root.openAbout();
                root.smokeStep = 6;
            } else if (root.smokeStep === 6) {
                if (pageStack.layers.depth !== 1) {
                    return;
                }
                root.closeLayers();
                root.smokeStep = 7;
                Controller.completeSmokeTest();
                smokeTimer.stop();
                Qt.quit();
            }
        }
    }
}

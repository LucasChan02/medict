// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    readonly property bool narrow: width < Kirigami.Units.gridUnit * 46

    title: appController.currentWord.length > 0
        ? qsTr("%1 — Medict").arg(appController.currentWord)
        : qsTr("Medict")
    width: 1180
    height: 780
    minimumWidth: Kirigami.Units.gridUnit * 24
    minimumHeight: Kirigami.Units.gridUnit * 22
    visible: true

    function closeLayers(): void {
        while (pageStack.layers.depth > 0) {
            pageStack.layers.pop();
        }
    }

    function focusLookup(): void {
        closeLayers();
        pageStack.currentItem.focusSearch();
    }

    function openSettings(): void {
        closeLayers();
        appController.loadSettings();
        pageStack.layers.push(settingsComponent);
    }

    function openNotebooks(): void {
        closeLayers();
        appController.loadNotebooks();
        pageStack.layers.push(notebooksComponent);
    }

    function openCssEditor(): void {
        closeLayers();
        if (appController.prepareCssEditor()) {
            pageStack.layers.push(cssEditorComponent);
        }
    }

    function openAbout(): void {
        closeLayers();
        pageStack.layers.push(aboutComponent);
    }

    globalDrawer: Kirigami.GlobalDrawer {
        id: drawer

        modal: root.narrow
        collapsible: !root.narrow
        width: Math.min(root.width * 0.82, Kirigami.Units.gridUnit * 21)
        title: qsTr("Medict")
        titleIcon: "qrc:/appicon.png"

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
                text: qsTr("Dictionaries")
                font.weight: Font.DemiBold
                Layout.topMargin: Kirigami.Units.mediumSpacing
            }

            Controls.TextField {
                id: dictionaryFilter
                Layout.fillWidth: true
                placeholderText: qsTr("Filter dictionaries")
                clearButtonEnabled: true
            }

            Repeater {
                model: appController.dictionaries.filter(dictionary => {
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
                    checkable: appController.multiMode
                    checked: modelData.selected
                    Accessible.description: modelData.tooltip

                    Controls.ToolTip.visible: hovered
                    Controls.ToolTip.text: modelData.tooltip

                    onClicked: appController.selectDictionary(modelData.id)
                }
            }

            Controls.Label {
                visible: appController.dictionaries.length === 0
                text: qsTr("No dictionaries found.")
                opacity: 0.7
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }

            Controls.CheckBox {
                Layout.topMargin: Kirigami.Units.smallSpacing
                text: qsTr("Look up in several dictionaries")
                checked: appController.multiMode
                onToggled: appController.setMultiMode(checked)
            }

            Kirigami.Separator {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                Layout.bottomMargin: Kirigami.Units.smallSpacing
            }

            Controls.Label {
                text: qsTr("Matches")
                font.weight: Font.DemiBold
            }

            Repeater {
                model: appController.matches

                delegate: Controls.ItemDelegate {
                    required property var modelData

                    Layout.fillWidth: true
                    text: modelData.keyword
                    onClicked: {
                        drawer.close();
                        appController.useSuggestion(modelData.keyword);
                    }
                }
            }

            Controls.Label {
                visible: appController.matches.length === 0
                text: qsTr("Start typing to see matches.")
                opacity: 0.7
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }

            Controls.Button {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                text: qsTr("Refresh dictionaries")
                icon.name: "view-refresh"
                enabled: appController.ready
                onClicked: appController.refreshDictionaries()
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
}

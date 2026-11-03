// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtWebEngine
import org.kde.kirigami as Kirigami
import medict_app

Kirigami.Page {
    id: page

    readonly property var appState: applicationWindow().state
    readonly property var definitions: appState.definitions || []
    readonly property var liveCss: applicationWindow().previewCss
    readonly property int selectedCount: (appState.selectedDictIds || []).length
    readonly property int dictionaryCount: (appState.dictionaries || []).length

    title: appState.currentWord.length > 0
        ? appState.currentWord
        : qsTr("Search")
    padding: 0

    function focusSearch(): void {
        searchField.forceActiveFocus();
        searchField.selectAll();
    }

    function queryParameter(urlText, name): string {
        const queryIndex = urlText.indexOf("?");
        if (queryIndex < 0) {
            return "";
        }
        const parts = urlText.substring(queryIndex + 1).split("&");
        for (let index = 0; index < parts.length; ++index) {
            const pair = parts[index];
            const equals = pair.indexOf("=");
            const key = equals < 0 ? pair : pair.substring(0, equals);
            if (decodeURIComponent(key) === name) {
                const value = equals < 0 ? "" : pair.substring(equals + 1);
                try {
                    return decodeURIComponent(value.replace(/\+/g, " "));
                } catch (error) {
                    return value;
                }
            }
        }
        return "";
    }

    function isLoopbackUrl(urlText): bool {
        return /^https?:\/\/(127\.0\.0\.1|localhost|\[::1\])(?::\d+)?\//i.test(urlText);
    }

    function handleNavigation(request): void {
        const urlText = request.url.toString();
        if (urlText.startsWith("entry://")) {
            let word = urlText.substring("entry://".length);
            try {
                word = decodeURIComponent(word);
            } catch (error) {
                // Use the original text when it is not valid percent encoding.
            }
            if (word.length > 0) {
                Controller.lookup(word.split("#")[0]);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }
        if (urlText.startsWith("medict-lookup://")) {
            const word = page.queryParameter(urlText, "word");
            if (word.length > 0) {
                Controller.lookup(word);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }
        if (urlText.startsWith("sound://")) {
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }
        if ((urlText.startsWith("http://") || urlText.startsWith("https://"))
                && !page.isLoopbackUrl(urlText)) {
            if (request.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) {
                Qt.openUrlExternally(request.url);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }
        request.action = WebEngineNavigationRequest.AcceptRequest;
    }

    function forEachView(callback): void {
        for (let index = 0; index < definitionRepeater.count; ++index) {
            const item = definitionRepeater.itemAt(index);
            if (item && item.webView) {
                callback(item.webView);
            }
        }
    }

    function applyPreviewCss(view): void {
        const css = applicationWindow().previewCss;
        const script = "(function(){"
            + "var old=document.getElementById('medict-live-preview-css');"
            + "if(old){old.remove();}"
            + "var value=" + JSON.stringify(css) + ";"
            + "if(value.length){"
            + "var style=document.createElement('style');"
            + "style.id='medict-live-preview-css';"
            + "style.textContent=value;"
            + "document.head.appendChild(style);"
            + "}"
            + "})();";
        view.runJavaScript(script);
    }

    function reloadDefinitions(): void {
        page.forEachView(view => view.reload());
    }

    actions: [
        Kirigami.Action {
            text: qsTr("Back")
            icon.name: "go-previous"
            shortcut: "Alt+Left"
            enabled: appState.canGoBack
            onTriggered: Controller.back()
        },
        Kirigami.Action {
            text: qsTr("Forward")
            icon.name: "go-next"
            shortcut: "Alt+Right"
            enabled: appState.canGoForward
            onTriggered: Controller.forward()
        },
        Kirigami.Action {
            text: qsTr("Reload")
            icon.name: "view-refresh"
            shortcut: "F5"
            enabled: appState.ready
            onTriggered: page.reloadDefinitions()
        },
        Kirigami.Action {
            text: qsTr("Multi-dictionary")
            icon.name: "view-list-details"
            checkable: true
            checked: appState.multiMode
            onTriggered: Controller.setMultiMode(checked)
        },
        Kirigami.Action {
            text: qsTr("Save word")
            icon.name: "bookmark-new"
            shortcut: "Ctrl+D"
            enabled: appState.currentWord.length > 0
            onTriggered: Controller.addBookmark()
        },
        Kirigami.Action {
            text: qsTr("Rebuild dictionary index")
            icon.name: "view-refresh"
            enabled: appState.currentDictId.length > 0
            onTriggered: Controller.rebuildIndex()
        },
        Kirigami.Action {
            text: qsTr("Edit dictionary CSS")
            icon.name: "document-edit"
            enabled: appState.currentDictId.length > 0
            onTriggered: applicationWindow().openCssEditor()
        },
        Kirigami.Action {
            text: qsTr("Export entry")
            icon.name: "document-export"
            enabled: appState.currentWord.length > 0
            onTriggered: exportDialog.open()
        },
        Kirigami.Action {
            text: qsTr("Zoom in")
            icon.name: "zoom-in"
            shortcut: "Ctrl+="
            onTriggered: Controller.setZoom(Math.min(160, appState.zoom + 10))
        },
        Kirigami.Action {
            text: qsTr("Zoom out")
            icon.name: "zoom-out"
            shortcut: "Ctrl+-"
            onTriggered: Controller.setZoom(Math.max(80, appState.zoom - 10))
        },
        Kirigami.Action {
            text: qsTr("Reset zoom")
            icon.name: "zoom-original"
            onTriggered: Controller.setZoom(100)
        }
    ]

    footer: Controls.Pane {
        visible: !Kirigami.Settings.isMobile
        padding: Kirigami.Units.smallSpacing

        RowLayout {
            anchors.fill: parent
            spacing: Kirigami.Units.mediumSpacing

            Controls.BusyIndicator {
                visible: appState.busy
                running: visible
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }

            Controls.Label {
                Layout.fillWidth: true
                text: appState.status
                elide: Text.ElideRight
            }

            Controls.Label {
                text: qsTr("%1 of %2 dictionaries").arg(page.selectedCount).arg(page.dictionaryCount)
                opacity: 0.7
            }
        }
    }

    Component.onCompleted: {
        searchField.text = appState.currentWord;
        Qt.callLater(page.focusSearch);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Controls.Pane {
            Layout.fillWidth: true
            padding: Kirigami.Units.largeSpacing

            RowLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.smallSpacing

                Controls.TextField {
                    id: searchField

                    Layout.fillWidth: true
                    placeholderText: qsTr("Search a word")
                    leftPadding: Kirigami.Units.iconSizes.smallMedium
                        + Kirigami.Units.smallSpacing * 2
                    rightPadding: searchField.text.length > 0
                        ? clearSearchButton.implicitWidth
                            + Kirigami.Units.smallSpacing * 2
                        : Kirigami.Units.smallSpacing * 2
                    selectByMouse: true

                    Kirigami.Icon {
                        anchors.left: parent.left
                        anchors.leftMargin: Kirigami.Units.smallSpacing
                        anchors.verticalCenter: parent.verticalCenter
                        width: Kirigami.Units.iconSizes.smallMedium
                        height: width
                        source: "edit-find"
                    }

                    Controls.ToolButton {
                        id: clearSearchButton

                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        visible: searchField.text.length > 0
                        display: Controls.AbstractButton.IconOnly
                        icon.name: "edit-clear"
                        text: qsTr("Clear search")
                        Accessible.name: text
                        Controls.ToolTip.visible: hovered
                        Controls.ToolTip.text: text
                        onClicked: {
                            searchField.clear();
                            searchField.forceActiveFocus();
                            suggestionTimer.stop();
                            Controller.suggest("");
                        }
                    }

                    onAccepted: {
                        suggestionPopup.close();
                        Controller.lookup(text);
                    }
                    onTextEdited: suggestionTimer.restart()
                }

                Controls.Button {
                    text: qsTr("Search")
                    icon.name: "edit-find"
                    enabled: searchField.text.trim().length > 0
                    onClicked: {
                        suggestionPopup.close();
                        Controller.lookup(searchField.text);
                    }
                }
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            Layout.bottomMargin: appState.error.length > 0
                ? Kirigami.Units.largeSpacing
                : 0
            visible: appState.error.length > 0
            type: Kirigami.MessageType.Error
            text: appState.error
            actions: [
                Kirigami.Action {
                    text: qsTr("Dismiss")
                    icon.name: "dialog-close"
                    onTriggered: Controller.dismissError()
                }
            ]
        }

        Controls.TabBar {
            id: definitionTabs

            Layout.fillWidth: true
            visible: page.definitions.length > 1

            Repeater {
                model: page.definitions

                delegate: Controls.TabButton {
                    required property var modelData
                    text: modelData.dictName
                }
            }
        }

        StackLayout {
            id: definitionStack

            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: page.definitions.length > 0
                ? Math.max(0, definitionTabs.currentIndex)
                : -1

            Repeater {
                id: definitionRepeater
                model: page.definitions

                delegate: Item {
                    id: definitionContainer

                    required property var modelData

                    property alias webView: definitionView

                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    WebEngineView {
                        id: definitionView

                        anchors.fill: parent
                        profile: persistentProfile
                        visible: definitionContainer.modelData.hasEntry
                        url: definitionContainer.modelData.hasEntry
                            ? definitionContainer.modelData.url
                            : ""
                        zoomFactor: page.appState.zoom / 100
                        settings.javascriptEnabled: true
                        settings.localStorageEnabled: true

                        onNavigationRequested: function(request) {
                            page.handleNavigation(request);
                        }
                        onLoadProgressChanged: {
                            if (loadProgress === 100) {
                                page.applyPreviewCss(definitionView);
                            }
                        }
                        onNewWindowRequested: function(request) {
                            if (request.requestedUrl.toString().length > 0) {
                                Qt.openUrlExternally(request.requestedUrl);
                            }
                        }
                    }

                    Kirigami.PlaceholderMessage {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.largeSpacing
                        visible: !definitionContainer.modelData.hasEntry
                        icon.name: "dialog-information"
                        text: definitionContainer.modelData.dictName
                        explanation: definitionContainer.modelData.message
                    }
                }
            }

            Kirigami.PlaceholderMessage {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: page.definitions.length === 0
                icon.name: "bookmarks"
                text: qsTr("Look up a word")
                explanation: appState.ready
                    ? qsTr("Type a word and press Enter. Select several dictionaries to compare definitions.")
                    : qsTr("The dictionary engine is starting.")

                helpfulAction: Kirigami.Action {
                    text: qsTr("Search")
                    icon.name: "edit-find"
                    onTriggered: page.focusSearch()
                }
            }
        }
    }

    Timer {
        id: suggestionTimer
        interval: 220
        onTriggered: Controller.suggest(searchField.text)
    }

    Controls.Popup {
        id: suggestionPopup

        parent: searchField
        x: 0
        y: searchField.height + Kirigami.Units.smallSpacing
        width: searchField.width
        padding: 0
        visible: (appState.suggestions || []).length > 0 && searchField.activeFocus
        closePolicy: Controls.Popup.CloseOnEscape
            | Controls.Popup.CloseOnPressOutside

        contentItem: ListView {
            implicitHeight: Math.min(contentHeight, Kirigami.Units.gridUnit * 16)
            clip: true
            model: appState.suggestions || []

            delegate: Controls.ItemDelegate {
                required property var modelData

                width: ListView.view.width
                text: modelData
                onClicked: {
                    searchField.text = modelData;
                    suggestionPopup.close();
                    Controller.lookup(modelData);
                }
            }
        }
    }

    FileDialog {
        id: exportDialog

        title: qsTr("Export entry")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "html"
        nameFilters: [qsTr("HTML files (*.html)")]
        onAccepted: Controller.exportEntry(Controller.localFile(selectedFile.toString()))
    }

    WebEngineProfile {
        id: persistentProfile

        storageName: "Medict"
        offTheRecord: false
    }

    Connections {
        target: applicationWindow()

        function onPreviewCssChanged(): void {
            page.forEachView(view => page.applyPreviewCss(view));
        }
    }
}

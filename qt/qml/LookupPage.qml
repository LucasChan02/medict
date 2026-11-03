// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtWebEngine
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: page

    property string currentCss: ""

    WebEngineProfile {
        id: persistentProfile

        storageName: "Medict"
        offTheRecord: false
    }

    readonly property string navigationBridge: `
        (function () {
            if (window.__medictKirigamiNavigationInstalled) {
                return;
            }
            window.__medictKirigamiNavigationInstalled = true;

            function jump(word) {
                if (!word) {
                    return;
                }
                var target = String(word).split('#')[0].trim();
                if (!target) {
                    return;
                }
                window.location.assign(window.location.origin
                    + '/__mdict/__medict_qjump?word=' + encodeURIComponent(target));
            }

            window.addEventListener('message', function (event) {
                var data = event.data || {};
                if (data.evtype === '__Medict_INNER_FRAME_MSG_EVTP_ENTRY_JUMP') {
                    jump(data.word);
                    return;
                }
                if (data.evtype === '__Medict_INNER_FRAME_MSG_EVTP_CLICK_LOOKUP') {
                    jump(data.word);
                }
            });
        })();
    `

    title: appController.currentWord.length > 0
        ? appController.currentWord
        : qsTr("Medict")
    padding: 0

    function focusSearch(): void {
        searchField.forceActiveFocus();
        searchField.selectAll();
    }

    function forEachView(callback): void {
        for (let index = 0; index < definitionRepeater.count; ++index) {
            const item = definitionRepeater.itemAt(index);
            if (item && item.webView) {
                callback(item.webView);
            }
        }
    }

    function cssScript(css): string {
        return "(function(){"
            + "var old=document.getElementById('medict-kirigami-user-css');"
            + "if(old){old.remove();}"
            + "var value=" + JSON.stringify(css) + ";"
            + "if(value.length){"
            + "var style=document.createElement('style');"
            + "style.id='medict-kirigami-user-css';"
            + "style.textContent=value;"
            + "document.head.appendChild(style);"
            + "}"
            + "})();";
    }

    function applyCss(view, css): void {
        view.runJavaScript(cssScript(css));
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
        const match = urlText.match(/^https?:\/\/([^/:?#]+)/i);
        if (!match) {
            return false;
        }
        const host = match[1].toLowerCase();
        return host === "127.0.0.1"
            || host === "localhost"
            || host === "[::1]"
            || host === "::1";
    }

    function handleNavigation(request): void {
        const urlText = request.url.toString();
        const basePath = urlText.split("?")[0];

        if (basePath.endsWith("/__medict_qjump")) {
            const word = queryParameter(urlText, "word");
            if (word.length > 0) {
                appController.lookup(word);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }

        if (urlText.startsWith("entry://")) {
            let word = urlText.substring("entry://".length);
            try {
                word = decodeURIComponent(word);
            } catch (error) {
                // Keep the original text when it is not valid percent encoding.
            }
            if (word.length > 0) {
                appController.lookup(word);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }

        if ((urlText.startsWith("http://") || urlText.startsWith("https://"))
            && !isLoopbackUrl(urlText)) {
            if (request.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) {
                Qt.openUrlExternally(request.url);
            }
            request.action = WebEngineNavigationRequest.IgnoreRequest;
            return;
        }

        request.action = WebEngineNavigationRequest.AcceptRequest;
    }

    actions: [
        Kirigami.Action {
            text: qsTr("Back")
            icon.name: "go-previous"
            shortcut: "Alt+Left"
            enabled: appController.canGoBack
            onTriggered: appController.back()
        },
        Kirigami.Action {
            text: qsTr("Forward")
            icon.name: "go-next"
            shortcut: "Alt+Right"
            enabled: appController.canGoForward
            onTriggered: appController.forward()
        },
        Kirigami.Action {
            text: qsTr("Reload")
            icon.name: "view-refresh"
            shortcut: "F5"
            enabled: appController.ready
            onTriggered: appController.reloadViews()
        },
        Kirigami.Action {
            text: qsTr("Multi-dictionary")
            icon.name: "view-list-details"
            checkable: true
            checked: appController.multiMode
            onTriggered: appController.setMultiMode(checked)
        },
        Kirigami.Action {
            text: qsTr("Save word")
            icon.name: "bookmark-new"
            shortcut: "Ctrl+D"
            enabled: appController.currentWord.length > 0
            onTriggered: appController.addBookmark()
        },
        Kirigami.Action {
            text: qsTr("Edit dictionary CSS")
            icon.name: "document-edit"
            enabled: appController.currentDictionaryId.length > 0
            onTriggered: applicationWindow().openCssEditor()
        },
        Kirigami.Action {
            text: qsTr("Export entry")
            icon.name: "document-export"
            enabled: appController.currentWord.length > 0
            onTriggered: exportDialog.open()
        },
        Kirigami.Action {
            text: qsTr("Zoom in")
            icon.name: "zoom-in"
            onTriggered: appController.zoomIn()
        },
        Kirigami.Action {
            text: qsTr("Zoom out")
            icon.name: "zoom-out"
            onTriggered: appController.zoomOut()
        },
        Kirigami.Action {
            text: qsTr("Reset zoom")
            icon.name: "zoom-original"
            onTriggered: appController.resetZoom()
        }
    ]

    footer: Controls.Pane {
        visible: !Kirigami.Settings.isMobile
        padding: Kirigami.Units.smallSpacing

        RowLayout {
            anchors.fill: parent
            spacing: Kirigami.Units.mediumSpacing

            Controls.Label {
                Layout.fillWidth: true
                text: appController.statusText
                elide: Text.ElideRight
            }

            Controls.Label {
                text: appController.dictionaryStatus
                opacity: 0.7
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Item {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            implicitHeight: searchRow.implicitHeight

            RowLayout {
                id: searchRow
                anchors.fill: parent
                spacing: Kirigami.Units.smallSpacing

                Controls.TextField {
                    id: searchField

                    Layout.fillWidth: true
                    placeholderText: qsTr("Search a word")
                    clearButtonEnabled: true
                    leftPadding: Kirigami.Units.iconSizes.smallMedium
                        + Kirigami.Units.smallSpacing * 2

                    Component.onCompleted: text = appController.searchText

                    onAccepted: {
                        suggestionPopup.close();
                        appController.lookup(text);
                    }
                    onTextEdited: {
                        appController.setSearchText(text);
                        suggestionTimer.restart();
                    }
                }

                Kirigami.Icon {
                    anchors.left: searchField.left
                    anchors.leftMargin: Kirigami.Units.smallSpacing
                    anchors.verticalCenter: searchField.verticalCenter
                    width: Kirigami.Units.iconSizes.smallMedium
                    height: width
                    source: "edit-find"
                }

                Controls.Button {
                    text: qsTr("Search")
                    icon.name: "edit-find"
                    enabled: searchField.text.trim().length > 0
                    onClicked: {
                        suggestionPopup.close();
                        appController.lookup(searchField.text);
                    }
                }
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            Layout.bottomMargin: appController.errorVisible
                ? Kirigami.Units.largeSpacing
                : 0
            visible: appController.errorVisible
            type: Kirigami.MessageType.Error
            text: appController.errorTitle + "\n" + appController.errorText
            actions: [
                Kirigami.Action {
                    text: qsTr("Dismiss")
                    icon.name: "dialog-close"
                    onTriggered: appController.dismissError()
                }
            ]
        }

        Controls.TabBar {
            id: definitionTabs

            Layout.fillWidth: true
            visible: appController.definitions.length > 1
            currentIndex: appController.currentDefinitionIndex
            onCurrentIndexChanged: {
                if (currentIndex !== appController.currentDefinitionIndex) {
                    appController.currentDefinitionIndex = currentIndex;
                }
            }

            Repeater {
                model: appController.definitions

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
            currentIndex: appController.currentDefinitionIndex

            Repeater {
                id: definitionRepeater
                model: appController.definitions

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
                        zoomFactor: appController.zoomFactor

                        settings.javascriptEnabled: true
                        settings.localStorageEnabled: true

                        onNavigationRequested: function(request) {
                            page.handleNavigation(request);
                        }
                        onLoadProgressChanged: {
                            if (loadProgress === 100) {
                                runJavaScript(page.navigationBridge);
                                if (page.currentCss.length > 0) {
                                    page.applyCss(definitionView, page.currentCss);
                                }
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
                visible: appController.definitions.length === 0
                icon.name: "bookmarks"
                text: qsTr("Look up a word")
                explanation: appController.ready
                    ? qsTr("Type a word in the search field and press Enter. "
                        + "Select several dictionaries to compare results.")
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
        onTriggered: appController.requestSuggestions(searchField.text)
    }

    Controls.Popup {
        id: suggestionPopup

        parent: searchField
        x: 0
        y: searchField.height + Kirigami.Units.smallSpacing
        width: searchField.width
        padding: 0
        visible: appController.suggestions.length > 0 && searchField.activeFocus
        closePolicy: Controls.Popup.CloseOnEscape
            | Controls.Popup.CloseOnPressOutside

        contentItem: ListView {
            implicitHeight: Math.min(contentHeight, Kirigami.Units.gridUnit * 16)
            clip: true
            model: appController.suggestions

            delegate: Controls.ItemDelegate {
                required property var modelData

                width: ListView.view.width
                text: modelData
                onClicked: {
                    appController.useSuggestion(modelData);
                    suggestionPopup.close();
                }
            }
        }
    }

    Connections {
        target: appController

        function onSearchTextChanged(): void {
            if (searchField.text !== appController.searchText) {
                searchField.text = appController.searchText;
            }
        }

        function onReloadRequested(): void {
            page.forEachView(view => view.reload());
        }

        function onApplyCssRequested(css): void {
            page.currentCss = css;
            page.forEachView(view => page.applyCss(view, css));
        }
    }

    FileDialog {
        id: exportDialog

        title: qsTr("Export entry")
        fileMode: FileDialog.SaveFile
        nameFilters: [qsTr("HTML files (*.html)")]

        onAccepted: appController.exportCurrentEntry(appController.localFile(selectedFile))
    }
}

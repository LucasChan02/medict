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
    property bool webEngineLoaded: false
    property var persistentProfile: null
    property bool suspended: false

    title: appState.currentWord.length > 0
        ? appState.currentWord
        : qsTr("Search")
    padding: 0

    function focusSearch(): void {
        searchField.forceActiveFocus();
        searchField.selectAll();
    }

    function setSuspended(value): void {
        page.suspended = value;
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
        BreezeAction {
            text: qsTr("Back")
            breezeIconName: "go-previous"
            shortcut: "Alt+Left"
            enabled: appState.canGoBack
            onTriggered: Controller.back()
        },
        BreezeAction {
            text: qsTr("Forward")
            breezeIconName: "go-next"
            shortcut: "Alt+Right"
            enabled: appState.canGoForward
            onTriggered: Controller.forward()
        },
        BreezeAction {
            text: qsTr("Reload")
            breezeIconName: "view-refresh"
            shortcut: "F5"
            enabled: appState.ready
            onTriggered: page.reloadDefinitions()
        },
        BreezeAction {
            text: qsTr("Multi-dictionary")
            breezeIconName: "view-list-details"
            checkable: true
            checked: appState.multiMode
            onTriggered: Controller.setMultiMode(checked)
        },
        BreezeAction {
            text: qsTr("Save word")
            breezeIconName: "bookmark-new"
            shortcut: "Ctrl+D"
            enabled: appState.currentWord.length > 0
            onTriggered: Controller.addBookmark()
        },
        BreezeAction {
            text: qsTr("Rebuild dictionary index")
            breezeIconName: "view-refresh"
            enabled: appState.currentDictId.length > 0
            onTriggered: Controller.rebuildIndex()
        },
        BreezeAction {
            text: qsTr("Edit dictionary CSS")
            breezeIconName: "document-edit"
            enabled: appState.currentDictId.length > 0
            onTriggered: applicationWindow().openCssEditor()
        },
        BreezeAction {
            text: qsTr("Export entry")
            breezeIconName: "document-export"
            enabled: appState.currentWord.length > 0
            onTriggered: exportDialog.open()
        },
        BreezeAction {
            text: qsTr("Zoom in")
            breezeIconName: "zoom-in"
            shortcut: "Ctrl+="
            onTriggered: Controller.setZoom(Math.min(160, appState.zoom + 10))
        },
        BreezeAction {
            text: qsTr("Zoom out")
            breezeIconName: "zoom-out"
            shortcut: "Ctrl+-"
            onTriggered: Controller.setZoom(Math.max(80, appState.zoom - 10))
        },
        BreezeAction {
            text: qsTr("Reset zoom")
            breezeIconName: "zoom-original"
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
        persistentProfile = persistentProfilePrototype.instance();
        if (Controller.smokeTest) {
            console.info("Medict smoke: WebEngine profile ready");
        }
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
                BreezeAction {
                    text: qsTr("Dismiss")
                    breezeIconName: "dialog-close"
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
                ? Math.min(
                    page.definitions.length - 1,
                    Math.max(0, definitionTabs.currentIndex))
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
                        visible: !page.suspended
                            && definitionContainer.modelData.hasEntry
                        url: definitionContainer.modelData.hasEntry
                            ? definitionContainer.modelData.url
                            : ""
                        zoomFactor: page.appState.zoom / 100
                        settings.javascriptEnabled: true
                        settings.localStorageEnabled: true

                        onNavigationRequested: function(request) {
                            page.handleNavigation(request);
                        }
                        onLoadingChanged: function(loadRequest) {
                            if (Controller.smokeTest) {
                                console.info("Medict smoke: WebEngine load status",
                                    loadRequest.status, loadRequest.url);
                            }
                            if (loadRequest.status === WebEngineView.LoadSucceededStatus) {
                                page.webEngineLoaded = true;
                            } else if (loadRequest.status === WebEngineView.LoadFailedStatus) {
                                console.error("Medict could not load a definition:",
                                    loadRequest.errorString);
                            }
                        }
                        onLoadProgressChanged: {
                            if (loadProgress === 100) {
                                if (Controller.smokeTest) {
                                    console.info("Medict smoke: WebEngine load complete");
                                }
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

                helpfulAction: BreezeAction {
                    text: qsTr("Search")
                    breezeIconName: "edit-find"
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

    WebEngineProfilePrototype {
        id: persistentProfilePrototype

        storageName: "Medict"
    }

    Connections {
        target: applicationWindow()

        function onPreviewCssChanged(): void {
            page.forEachView(view => page.applyPreviewCss(view));
        }
    }
}

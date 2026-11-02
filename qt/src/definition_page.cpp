// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "definition_page.h"

#include <QDesktopServices>
#include <QUrlQuery>

DefinitionPage::DefinitionPage(QObject *parent)
    : QWebEnginePage(parent)
{
    connect(this, &QWebEnginePage::loadFinished,
            this, &DefinitionPage::installNavigationBridge);
}

bool DefinitionPage::acceptNavigationRequest(const QUrl &url, NavigationType type, bool isMainFrame)
{
    Q_UNUSED(isMainFrame)

    if (url.path().endsWith(QStringLiteral("/__medict_qjump"))) {
        const QUrlQuery query(url);
        emit wordRequested(query.queryItemValue(QStringLiteral("word"), QUrl::FullyDecoded));
        return false;
    }

    if (url.scheme() == QStringLiteral("entry")) {
        QString word = url.toString();
        word.remove(0, QStringLiteral("entry://").size());
        emit wordRequested(word);
        return false;
    }

    if (url.scheme() == QStringLiteral("http") || url.scheme() == QStringLiteral("https")) {
        const QString host = url.host();
        const bool loopback = host == QStringLiteral("127.0.0.1")
            || host == QStringLiteral("localhost")
            || host == QStringLiteral("::1");
        if (loopback) {
            return true;
        }
        if (type == NavigationTypeLinkClicked) {
            QDesktopServices::openUrl(url);
        }
        return false;
    }

    return true;
}

void DefinitionPage::installNavigationBridge(bool ok)
{
    if (!ok) {
        return;
    }
    runJavaScript(QStringLiteral(R"JS(
(function () {
    if (window.__medictQtNavigationInstalled) {
        return;
    }
    window.__medictQtNavigationInstalled = true;

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
)JS"));
}

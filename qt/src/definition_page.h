// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QUrl>
#include <QWebEnginePage>

class DefinitionPage final : public QWebEnginePage
{
    Q_OBJECT

public:
    explicit DefinitionPage(QObject *parent = nullptr);

signals:
    void wordRequested(const QString &word);

protected:
    bool acceptNavigationRequest(const QUrl &url, NavigationType type, bool isMainFrame) override;

private:
    void installNavigationBridge(bool ok);
};


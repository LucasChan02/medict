// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "api_client.h"
#include "main_window.h"

#include <QApplication>
#include <QIcon>
#include <QTimer>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("Medict"));
    QApplication::setApplicationDisplayName(QStringLiteral("Medict"));
    QApplication::setOrganizationName(QStringLiteral("Medict"));
    QApplication::setApplicationVersion(QStringLiteral(MEDICT_VERSION));
    QApplication::setWindowIcon(QIcon(QStringLiteral(":/appicon.png")));

    auto *api = new ApiClient(&app);
    auto *window = new MainWindow(api, QStringLiteral(MEDICT_VERSION));
    window->show();

    QTimer::singleShot(0, api, [api]() {
        api->start(QStringLiteral(MEDICT_VERSION));
    });

    return app.exec();
}


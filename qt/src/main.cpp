// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "api_client.h"
#include "app_controller.h"

#include <QDebug>
#include <QFileInfo>
#include <QGuiApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickStyle>
#include <QtWebEngineQuick/QtWebEngineQuick>

namespace {

bool hasQuickStyle(const QString &styleName)
{
    QQmlEngine engine;
    const QString stylePath = QString(styleName).replace(QLatin1Char('.'), QLatin1Char('/'));
    const QStringList importPaths = engine.importPathList();
    for (const QString &importPath : importPaths) {
        if (QFileInfo::exists(importPath + QLatin1Char('/') + stylePath
                              + QStringLiteral("/qmldir"))) {
            return true;
        }
    }
    return false;
}

void configureTheme()
{
    const QString appDir = QCoreApplication::applicationDirPath();
    QStringList iconPaths{
        QStringLiteral(":/icons"),
        appDir + QStringLiteral("/share/icons"),
    };
    iconPaths.append(QIcon::themeSearchPaths());
    QIcon::setThemeSearchPaths(iconPaths);
    QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    if (QIcon::themeName().isEmpty()) {
        QIcon::setThemeName(QStringLiteral("breeze"));
    }

    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(hasQuickStyle(QStringLiteral("org.kde.desktop"))
                                  ? QStringLiteral("org.kde.desktop")
                                  : QStringLiteral("Fusion"));
    }
}

} // namespace

int main(int argc, char *argv[])
{
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
    QtWebEngineQuick::initialize();
    QGuiApplication app(argc, argv);
    QGuiApplication::setApplicationName(QStringLiteral("Medict"));
    QGuiApplication::setApplicationDisplayName(QStringLiteral("Medict"));
    QGuiApplication::setApplicationVersion(QStringLiteral(MEDICT_VERSION));
    QGuiApplication::setOrganizationName(QStringLiteral("Medict"));
    QGuiApplication::setOrganizationDomain(QStringLiteral("terasum.github.io"));
    QGuiApplication::setDesktopFileName(QStringLiteral("io.github.terasum.Medict"));
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/appicon.png")));

    configureTheme();

    ApiClient api;
    AppController controller(&api, QStringLiteral(MEDICT_VERSION));

    QQmlApplicationEngine engine;
    engine.addImportPath(QCoreApplication::applicationDirPath() + QStringLiteral("/qml"));
    engine.rootContext()->setContextProperty(QStringLiteral("appController"), &controller);
    engine.loadFromModule(QStringLiteral("Medict"), QStringLiteral("Main"));
    if (engine.rootObjects().isEmpty()) {
        return 1;
    }
    if (app.arguments().contains(QStringLiteral("--smoke-test"))) {
        qInfo("Medict QML smoke test passed.");
        return 0;
    }

    controller.start();
    return app.exec();
}


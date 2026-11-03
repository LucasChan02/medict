// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "api_client.h"
#include "app_controller.h"

#include <QDebug>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QIcon>
#include <QMutex>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickStyle>
#include <QtWebEngineQuick/QtWebEngineQuick>

namespace {

QString smokeTestLogPath;
QMutex smokeTestLogMutex;
QtMessageHandler previousMessageHandler = nullptr;

const char *messageTypeName(QtMsgType type)
{
    switch (type) {
    case QtDebugMsg:
        return "debug";
    case QtInfoMsg:
        return "info";
    case QtWarningMsg:
        return "warning";
    case QtCriticalMsg:
        return "critical";
    case QtFatalMsg:
        return "fatal";
    }
    return "message";
}

void smokeTestMessageHandler(QtMsgType type,
                             const QMessageLogContext &context,
                             const QString &message)
{
    {
        QMutexLocker locker(&smokeTestLogMutex);
        QFile logFile(smokeTestLogPath);
        if (logFile.open(QIODevice::WriteOnly | QIODevice::Append | QIODevice::Text)) {
            logFile.write("[");
            logFile.write(messageTypeName(type));
            logFile.write("] ");
            logFile.write(message.toUtf8());
            logFile.write("\n");
            logFile.flush();
        }
    }

    if (previousMessageHandler) {
        previousMessageHandler(type, context, message);
    }
}

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

    const bool smokeTest = app.arguments().contains(QStringLiteral("--smoke-test"));
    if (smokeTest) {
        smokeTestLogPath = app.applicationDirPath() + QStringLiteral("/smoke-test.log");
        QFile::remove(smokeTestLogPath);
        previousMessageHandler = qInstallMessageHandler(smokeTestMessageHandler);
    }

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
    if (smokeTest) {
        qInfo("Medict QML smoke test passed.");
        return 0;
    }

    controller.start();
    return app.exec();
}


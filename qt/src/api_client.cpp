// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "api_client.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPointer>
#include <QProcess>
#include <QProcessEnvironment>
#include <QStandardPaths>
#include <QUrl>

#include <utility>

ApiClient::ApiClient(QObject *parent)
    : QObject(parent)
    , m_process(new QProcess(this))
    , m_network(new QNetworkAccessManager(this))
{
    m_process->setProcessChannelMode(QProcess::SeparateChannels);

    connect(m_process, &QProcess::readyReadStandardOutput,
            this, &ApiClient::consumeStandardOutput);
    connect(m_process, &QProcess::readyReadStandardError,
            this, &ApiClient::consumeStandardError);
    connect(m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (m_ready || m_failureReported) {
            return;
        }
        m_failureReported = true;
        emit failed(tr("The dictionary backend did not start: %1").arg(
            error == QProcess::FailedToStart ? m_process->errorString() : QString::fromUtf8(m_stderrBuffer)));
    });
    connect(m_process, &QProcess::finished, this,
            [this](int exitCode, QProcess::ExitStatus status) {
        if (!m_ready || status == QProcess::CrashExit) {
            if (!m_failureReported) {
                m_failureReported = true;
                emit failed(tr("The dictionary backend stopped with exit code %1.\n%2")
                                .arg(exitCode)
                                .arg(QString::fromUtf8(m_stderrBuffer)));
            }
            return;
        }
        emit backendStopped(tr("The dictionary backend stopped."));
    });
}

ApiClient::~ApiClient()
{
    stop();
}

void ApiClient::start(const QString &version)
{
    Q_UNUSED(version);

    if (m_process->state() != QProcess::NotRunning) {
        return;
    }

    const QString executable = backendPath();
    if (executable.isEmpty()) {
        m_failureReported = true;
        emit failed(tr("The medictd backend binary is missing from this package."));
        return;
    }

    m_stdoutBuffer.clear();
    m_stderrBuffer.clear();
    m_ready = false;
    m_failureReported = false;

    QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("GIN_MODE"), QStringLiteral("release"));
    m_process->setProcessEnvironment(environment);
    m_process->start(executable, {QStringLiteral("--port"), QStringLiteral("0")});
}

void ApiClient::stop()
{
    if (m_process->state() == QProcess::NotRunning) {
        return;
    }
    m_process->closeWriteChannel();
    if (!m_process->waitForFinished(1500)) {
        m_process->terminate();
    }
    if (!m_process->waitForFinished(1200)) {
        m_process->kill();
        m_process->waitForFinished(1000);
    }
}

void ApiClient::get(const QString &path, JsonCallback callback, QObject *context)
{
    sendRequest("GET", path, {}, std::move(callback), context);
}

void ApiClient::post(const QString &path, const QJsonObject &body, JsonCallback callback,
                     QObject *context)
{
    sendRequest("POST", path, QJsonDocument(body).toJson(QJsonDocument::Compact),
                std::move(callback), context);
}

void ApiClient::sendRequest(const QByteArray &verb, const QString &path,
                            const QByteArray &body, JsonCallback callback, QObject *context)
{
    if (!m_ready || m_apiBaseUrl.isEmpty()) {
        callback(QJsonValue(), tr("The dictionary backend is not ready."));
        return;
    }

    QNetworkRequest request(QUrl(m_apiBaseUrl + path));
    request.setRawHeader("X-Medict-Token", m_token.toUtf8());
    request.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("Medict-Qt/") + m_backendVersion);
    request.setTransferTimeout(10 * 60 * 1000);

    QNetworkReply *reply = nullptr;
    if (verb == "GET") {
        reply = m_network->get(request);
    } else {
        request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
        reply = m_network->post(request, body);
    }
    handleReply(reply, std::move(callback), context);
}

void ApiClient::handleReply(QNetworkReply *reply, JsonCallback callback, QObject *context)
{
    const QPointer<QObject> guard(context);
    const bool guarded = context != nullptr;
    connect(reply, &QNetworkReply::finished, this,
            [reply, callback = std::move(callback), guard, guarded]() {
        reply->deleteLater();
        if (guarded && guard.isNull()) {
            return;
        }
        const QByteArray payload = reply->readAll();
        if (reply->error() != QNetworkReply::NoError) {
            callback(QJsonValue(), reply->errorString());
            return;
        }

        QJsonParseError parseError{};
        const QJsonDocument document = QJsonDocument::fromJson(payload, &parseError);
        if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
            callback(QJsonValue(), tr("The backend returned an invalid response."));
            return;
        }

        const QJsonObject response = document.object();
        const int code = response.value(QStringLiteral("code")).toInt();
        const QString error = response.value(QStringLiteral("err")).toString();
        if (code != 200) {
            callback(QJsonValue(), error.isEmpty() ? tr("The backend reported an error.") : error);
            return;
        }
        callback(response.value(QStringLiteral("data")), QString());
    });
}

void ApiClient::consumeStandardOutput()
{
    m_stdoutBuffer += m_process->readAllStandardOutput();
    int newline = -1;
    while ((newline = m_stdoutBuffer.indexOf('\n')) >= 0) {
        const QByteArray line = m_stdoutBuffer.left(newline).trimmed();
        m_stdoutBuffer.remove(0, newline + 1);
        if (!line.startsWith("MEDICT_READY ")) {
            continue;
        }

        QJsonParseError parseError{};
        const QJsonDocument document =
            QJsonDocument::fromJson(line.mid(QByteArrayLiteral("MEDICT_READY ").size()), &parseError);
        if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
            m_failureReported = true;
            emit failed(tr("The backend sent an invalid ready message."));
            return;
        }

        const QJsonObject ready = document.object();
        m_token = ready.value(QStringLiteral("token")).toString();
        m_staticBaseUrl = ready.value(QStringLiteral("url")).toString();
        m_apiBaseUrl = ready.value(QStringLiteral("apiUrl")).toString();
        m_configDir = ready.value(QStringLiteral("configDir")).toString();
        m_backendVersion = ready.value(QStringLiteral("version")).toString();
        if (m_token.isEmpty() || m_staticBaseUrl.isEmpty() || m_apiBaseUrl.isEmpty()) {
            m_failureReported = true;
            emit failed(tr("The backend ready message is incomplete."));
            return;
        }
        m_ready = true;
        emit ready();
        return;
    }
}

void ApiClient::consumeStandardError()
{
    m_stderrBuffer += m_process->readAllStandardError();
    constexpr int maximumLogSize = 128 * 1024;
    if (m_stderrBuffer.size() > maximumLogSize) {
        m_stderrBuffer = m_stderrBuffer.right(maximumLogSize);
    }
}

QString ApiClient::backendPath() const
{
    const QString name =
#ifdef Q_OS_WIN
        QStringLiteral("medictd.exe");
#else
        QStringLiteral("medictd");
#endif

    const QDir appDir(QCoreApplication::applicationDirPath());
    const QStringList candidates = {
        appDir.filePath(name),
        appDir.filePath(QStringLiteral("../libexec/") + name),
        appDir.filePath(QStringLiteral("../lib/medict/") + name),
    };
    for (const QString &candidate : candidates) {
        const QFileInfo info(candidate);
        if (info.isFile() && info.isExecutable()) {
            return info.absoluteFilePath();
        }
    }

    const QString inPath = QStandardPaths::findExecutable(QStringLiteral("medictd"));
    return inPath;
}

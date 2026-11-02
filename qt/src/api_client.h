// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QJsonObject>
#include <QJsonValue>
#include <QObject>
#include <QString>

#include <functional>

class QNetworkAccessManager;
class QNetworkReply;
class QProcess;

class ApiClient final : public QObject
{
    Q_OBJECT

public:
    using JsonCallback = std::function<void(const QJsonValue &data, const QString &error)>;

    explicit ApiClient(QObject *parent = nullptr);
    ~ApiClient() override;

    void start(const QString &version);
    void stop();

    bool isReady() const { return m_ready; }
    QString staticBaseUrl() const { return m_staticBaseUrl; }
    QString apiBaseUrl() const { return m_apiBaseUrl; }
    QString configDir() const { return m_configDir; }
    QString backendVersion() const { return m_backendVersion; }

    void get(const QString &path, JsonCallback callback, QObject *context = nullptr);
    void post(const QString &path, const QJsonObject &body, JsonCallback callback,
              QObject *context = nullptr);

signals:
    void ready();
    void failed(const QString &message);
    void backendStopped(const QString &message);

private:
    void consumeStandardOutput();
    void consumeStandardError();
    void sendRequest(const QByteArray &verb, const QString &path,
                     const QByteArray &body, JsonCallback callback, QObject *context);
    void handleReply(QNetworkReply *reply, JsonCallback callback, QObject *context);
    QString backendPath() const;

    QProcess *m_process = nullptr;
    QNetworkAccessManager *m_network = nullptr;
    QByteArray m_stdoutBuffer;
    QByteArray m_stderrBuffer;
    QString m_token;
    QString m_staticBaseUrl;
    QString m_apiBaseUrl;
    QString m_configDir;
    QString m_backendVersion;
    bool m_ready = false;
    bool m_failureReported = false;
};


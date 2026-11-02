// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "css_editor_dialog.h"

#include "api_client.h"

#include <QDialogButtonBox>
#include <QFont>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTimer>
#include <QUrlQuery>
#include <QVBoxLayout>

CssEditorDialog::CssEditorDialog(ApiClient *api, const QString &dictId,
                                 const QString &dictName, const QString &word,
                                 QWidget *parent)
    : QDialog(parent)
    , m_api(api)
    , m_dictId(dictId)
    , m_dictName(dictName)
    , m_word(word)
{
    setWindowTitle(tr("CSS editor — %1").arg(dictName));
    resize(760, 600);

    auto *layout = new QVBoxLayout(this);
    auto *hint = new QLabel(
        tr("This CSS override applies only to this dictionary. "
           "The preview updates while you type."),
        this);
    hint->setWordWrap(true);
    layout->addWidget(hint);

    m_editor = new QPlainTextEdit(this);
    m_editor->setLineWrapMode(QPlainTextEdit::NoWrap);
    m_editor->setFont(QFont(QStringLiteral("monospace"), 10));
    layout->addWidget(m_editor, 1);

    m_status = new QLabel(tr("Loading CSS..."), this);
    layout->addWidget(m_status);

    auto *buttonRow = new QHBoxLayout;
    auto *restoreButton = new QPushButton(tr("Restore dictionary CSS"), this);
    auto *removeButton = new QPushButton(tr("Remove override"), this);
    auto *saveButton = new QPushButton(tr("Save"), this);
    auto *closeButton = new QPushButton(tr("Close"), this);
    buttonRow->addWidget(restoreButton);
    buttonRow->addWidget(removeButton);
    buttonRow->addStretch();
    buttonRow->addWidget(saveButton);
    buttonRow->addWidget(closeButton);
    layout->addLayout(buttonRow);

    m_previewTimer = new QTimer(this);
    m_previewTimer->setSingleShot(true);
    m_previewTimer->setInterval(300);
    connect(m_previewTimer, &QTimer::timeout, this, [this]() {
        emit previewCss(m_editor->toPlainText());
    });
    connect(m_editor, &QPlainTextEdit::textChanged,
            this, &CssEditorDialog::schedulePreview);
    connect(restoreButton, &QPushButton::clicked,
            this, &CssEditorDialog::restoreBaseCss);
    connect(removeButton, &QPushButton::clicked,
            this, &CssEditorDialog::removeOverride);
    connect(saveButton, &QPushButton::clicked, this, &CssEditorDialog::saveCss);
    connect(closeButton, &QPushButton::clicked, this, &QDialog::reject);

    loadCss();
}

void CssEditorDialog::loadCss()
{
    QUrlQuery query;
    query.addQueryItem(QStringLiteral("dictId"), m_dictId);
    query.addQueryItem(QStringLiteral("word"), m_word);
    const QString path = QStringLiteral("/css?") + query.query(QUrl::FullyEncoded);
    m_api->get(path, [this](const QJsonValue &data, const QString &error) {
        if (!error.isEmpty() || !data.isObject()) {
            m_status->setText(error.isEmpty() ? tr("Cannot load the dictionary CSS.") : error);
            return;
        }
        const QJsonObject payload = data.toObject();
        const QString saved = payload.value(QStringLiteral("saved")).toString();
        m_baseCss = payload.value(QStringLiteral("base")).toString();
        m_editor->setPlainText(saved.isEmpty() ? m_baseCss : saved);
        m_status->setText(saved.isEmpty()
            ? tr("The editor shows the dictionary CSS. Changes become an override.")
            : tr("The editor shows the saved override."));
    }, this);
}

void CssEditorDialog::saveCss()
{
    m_api->post(QStringLiteral("/css"), QJsonObject{
        {QStringLiteral("dictId"), m_dictId},
        {QStringLiteral("css"), m_editor->toPlainText()},
    }, [this](const QJsonValue &, const QString &error) {
        m_status->setText(error.isEmpty() ? tr("CSS saved.") : error);
    }, this);
}

void CssEditorDialog::removeOverride()
{
    m_api->post(QStringLiteral("/css"), QJsonObject{
        {QStringLiteral("dictId"), m_dictId},
        {QStringLiteral("css"), QString()},
    }, [this](const QJsonValue &, const QString &error) {
        if (!error.isEmpty()) {
            m_status->setText(error);
            return;
        }
        m_editor->setPlainText(m_baseCss);
        emit previewCss(m_baseCss);
        m_status->setText(tr("The dictionary CSS override was removed."));
    }, this);
}

void CssEditorDialog::restoreBaseCss()
{
    m_editor->setPlainText(m_baseCss);
    emit previewCss(m_baseCss);
    m_status->setText(tr("The dictionary CSS was restored. Save to keep this result."));
}

void CssEditorDialog::schedulePreview()
{
    m_previewTimer->start();
}

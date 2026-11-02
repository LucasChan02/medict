// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QDialog>

class ApiClient;
class QLabel;
class QPlainTextEdit;
class QTimer;

class CssEditorDialog final : public QDialog
{
    Q_OBJECT

public:
    CssEditorDialog(ApiClient *api, const QString &dictId, const QString &dictName,
                    const QString &word, QWidget *parent = nullptr);

signals:
    void previewCss(const QString &css);

private:
    void loadCss();
    void saveCss();
    void removeOverride();
    void restoreBaseCss();
    void schedulePreview();

    ApiClient *m_api = nullptr;
    QString m_dictId;
    QString m_dictName;
    QString m_word;
    QString m_baseCss;
    QPlainTextEdit *m_editor = nullptr;
    QLabel *m_status = nullptr;
    QTimer *m_previewTimer = nullptr;
};


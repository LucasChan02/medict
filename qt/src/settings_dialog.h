// Copyright (C) 2026 Medict contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <QDialog>

class ApiClient;
class QCheckBox;
class QLabel;
class QLineEdit;
class QProgressBar;
class QPushButton;
class QSpinBox;

class SettingsDialog final : public QDialog
{
    Q_OBJECT

public:
    SettingsDialog(ApiClient *api, const QString &baseDictDir, QWidget *parent = nullptr);

signals:
    void dictionariesChanged();

private:
    void buildUi();
    void loadPreferences();
    void loadEcdictStatus();
    void savePreferences();
    void installEcdict();
    void setEcdictBusy(bool busy);

    ApiClient *m_api = nullptr;
    QString m_baseDictDir;
    QLineEdit *m_dictPath = nullptr;
    QCheckBox *m_allowSymlinks = nullptr;
    QSpinBox *m_groupLimit = nullptr;
    QLabel *m_ecdictStatus = nullptr;
    QPushButton *m_installButton = nullptr;
    QProgressBar *m_progress = nullptr;
};


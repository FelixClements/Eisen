package com.example.myapplication.data.reminder

import android.Manifest
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.example.myapplication.MainActivity
import com.example.myapplication.data.local.DatabaseProvider
import com.example.myapplication.data.local.toDomain

class TaskReminderWorker(
    appContext: Context,
    params: WorkerParameters,
) : CoroutineWorker(appContext, params) {
    override suspend fun doWork(): Result {
        val taskId = inputData.getLong(KEY_TASK_ID, NO_TASK_ID)
        val expectedReminderAt = inputData.getLong(KEY_EXPECTED_REMINDER_AT, NO_REMINDER)
        if (taskId == NO_TASK_ID || expectedReminderAt == NO_REMINDER) return Result.success()

        val taskDao = DatabaseProvider.getDatabase(applicationContext).taskDao()
        val task = taskDao
            .getTaskByIdNow(taskId)
            ?.toDomain()
            ?: return Result.success()

        if (
            task.isCompleted ||
            task.isArchived ||
            task.reminderAt != expectedReminderAt
        ) {
            return Result.success()
        }

        if (expectedReminderAt > System.currentTimeMillis()) return Result.retry()

        if (
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(
                applicationContext,
                Manifest.permission.POST_NOTIFICATIONS,
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            return Result.success()
        }

        TaskReminderNotifications.createChannel(applicationContext)
        val taskBeforePosting = taskDao.getTaskByIdNow(taskId)
            ?.toDomain()
            ?: return Result.success()
        if (
            taskBeforePosting.isCompleted ||
            taskBeforePosting.isArchived ||
            taskBeforePosting.reminderAt != expectedReminderAt
        ) {
            return Result.success()
        }

        if (expectedReminderAt > System.currentTimeMillis()) return Result.retry()

        val openAppIntent = PendingIntent.getActivity(
            applicationContext,
            TaskReminderNotifications.notificationId(taskId),
            Intent(applicationContext, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(
            applicationContext,
            TaskReminderNotifications.CHANNEL_ID,
        )
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle("Task reminder")
            .setContentText(taskBeforePosting.title)
            .setContentIntent(openAppIntent)
            .setAutoCancel(true)
            .build()

        TaskReminderNotifications.cancel(applicationContext, taskId)
        androidx.core.app.NotificationManagerCompat.from(applicationContext)
            .notify(TaskReminderNotifications.notificationId(taskId), notification)
        return Result.success()
    }

    companion object {
        const val KEY_TASK_ID = "taskId"
        const val KEY_EXPECTED_REMINDER_AT = "expectedReminderAt"
        private const val NO_TASK_ID = Long.MIN_VALUE
        private const val NO_REMINDER = Long.MIN_VALUE
    }
}

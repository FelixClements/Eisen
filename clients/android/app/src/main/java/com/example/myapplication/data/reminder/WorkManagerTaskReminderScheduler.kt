package com.example.myapplication.data.reminder

import android.content.Context
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import com.example.myapplication.domain.TaskReminderScheduler
import java.util.concurrent.TimeUnit

class WorkManagerTaskReminderScheduler(
    private val context: Context,
    private val workManager: WorkManager = WorkManager.getInstance(context.applicationContext),
    private val clock: () -> Long = { System.currentTimeMillis() },
) : TaskReminderScheduler {
    override fun schedule(taskId: Long, reminderAt: Long) {
        val request = OneTimeWorkRequestBuilder<TaskReminderWorker>()
            .setInitialDelay((reminderAt - clock()).coerceAtLeast(0L), TimeUnit.MILLISECONDS)
            .setInputData(
                Data.Builder()
                    .putLong(TaskReminderWorker.KEY_TASK_ID, taskId)
                    .putLong(TaskReminderWorker.KEY_EXPECTED_REMINDER_AT, reminderAt)
                    .build()
            )
            .build()

        workManager.enqueueUniqueWork(
            uniqueWorkName(taskId),
            ExistingWorkPolicy.REPLACE,
            request,
        )
    }

    override fun cancel(taskId: Long) {
        workManager.cancelUniqueWork(uniqueWorkName(taskId))
        TaskReminderNotifications.cancel(context, taskId)
    }

    private fun uniqueWorkName(taskId: Long): String = "task-reminder:$taskId"
}

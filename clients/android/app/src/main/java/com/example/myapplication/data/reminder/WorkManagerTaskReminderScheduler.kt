package com.example.myapplication.data.reminder

import android.content.Context
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkInfo
import com.example.myapplication.domain.TaskReminderScheduler
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import java.util.concurrent.TimeUnit

class WorkManagerTaskReminderScheduler(
    private val context: Context,
    private val workManager: WorkManager = WorkManager.getInstance(context.applicationContext),
    private val clock: () -> Long = { System.currentTimeMillis() },
) : TaskReminderScheduler {
    override fun schedule(taskId: Long, reminderAt: Long): Result<Unit> {
        return try {
            val request = OneTimeWorkRequestBuilder<TaskReminderWorker>()
                .setInitialDelay((reminderAt - clock()).coerceAtLeast(0L), TimeUnit.MILLISECONDS)
                .addTag(TAG_REMINDER)
                .addTag(taskIdTag(taskId))
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
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    override fun cancel(taskId: Long): Result<Unit> {
        return try {
            workManager.cancelUniqueWork(uniqueWorkName(taskId))
            TaskReminderNotifications.cancel(context, taskId)
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    override fun getScheduledTaskIds(): Flow<Set<Long>> {
        return workManager.getWorkInfosByTagFlow(TAG_REMINDER)
            .map { workInfos ->
                workInfos
                    .filter { it.state == WorkInfo.State.ENQUEUED || it.state == WorkInfo.State.RUNNING }
                    .mapNotNull { info ->
                        info.tags.firstOrNull { it.startsWith(PREFIX_TASK_ID) }
                            ?.removePrefix(PREFIX_TASK_ID)
                            ?.toLongOrNull()
                    }
                    .toSet()
            }
    }

    private fun uniqueWorkName(taskId: Long): String = "task-reminder:$taskId"
    private fun taskIdTag(taskId: Long): String = "$PREFIX_TASK_ID$taskId"

    companion object {
        private const val TAG_REMINDER = "task-reminder"
        private const val PREFIX_TASK_ID = "taskId:"
    }
}

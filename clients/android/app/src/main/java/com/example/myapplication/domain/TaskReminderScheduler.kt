package com.example.myapplication.domain

interface TaskReminderScheduler {
    fun schedule(taskId: Long, reminderAt: Long)

    /** Cancels both pending work and any notification already posted for this task. */
    fun cancel(taskId: Long)
}

object NoOpTaskReminderScheduler : TaskReminderScheduler {
    override fun schedule(taskId: Long, reminderAt: Long) = Unit

    override fun cancel(taskId: Long) = Unit
}

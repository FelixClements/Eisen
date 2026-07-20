package com.example.myapplication.domain

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf

interface TaskReminderScheduler {
    fun schedule(taskId: Long, reminderAt: Long): Result<Unit>

    /** Cancels both pending work and any notification already posted for this task. */
    fun cancel(taskId: Long): Result<Unit>

    /** Returns a flow of task IDs that currently have a scheduled reminder. */
    fun getScheduledTaskIds(): Flow<Set<Long>>
}

object NoOpTaskReminderScheduler : TaskReminderScheduler {
    override fun schedule(taskId: Long, reminderAt: Long) = Result.success(Unit)

    override fun cancel(taskId: Long) = Result.success(Unit)

    override fun getScheduledTaskIds(): Flow<Set<Long>> = flowOf(emptySet())
}

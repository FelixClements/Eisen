package com.example.myapplication.ui.util

import android.content.Context
import java.text.DateFormat
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import java.util.TimeZone

object DateTimeUtils {
    private const val DAY_IN_MILLIS = 24L * 60L * 60L * 1000L

    fun startOfDay(millis: Long): Long = Calendar.getInstance().apply {
        timeInMillis = millis
        set(Calendar.HOUR_OF_DAY, 0)
        set(Calendar.MINUTE, 0)
        set(Calendar.SECOND, 0)
        set(Calendar.MILLISECOND, 0)
    }.timeInMillis

    fun formatDueDate(dueAtMillis: Long, now: Long): String {
        val todayStart = startOfDay(now)
        val dueDayStart = startOfDay(dueAtMillis)

        return when {
            dueDayStart < todayStart -> "Overdue"
            dueDayStart == todayStart -> "Due today"
            dueDayStart == todayStart + DAY_IN_MILLIS -> "Due tomorrow"
            else -> "Due ${SimpleDateFormat("EEE, MMM d", Locale.getDefault()).format(Date(dueAtMillis))}"
        }
    }

    fun formatReminderAt(reminderAt: Long, context: Context): String {
        val is24Hour = android.text.format.DateFormat.is24HourFormat(context)
        val datePart = DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date(reminderAt))
        val timePattern = if (is24Hour) "HH:mm" else "h:mm a"
        val timePart = SimpleDateFormat(timePattern, Locale.getDefault()).format(Date(reminderAt))
        
        val now = System.currentTimeMillis()
        val todayStart = startOfDay(now)
        val reminderDayStart = startOfDay(reminderAt)
        
        val relativeDate = when {
            reminderDayStart == todayStart -> "Today"
            reminderDayStart == todayStart + DAY_IN_MILLIS -> "Tomorrow"
            else -> datePart
        }
        
        return "$relativeDate at $timePart"
    }

    fun datePickerMillisToLocalNoon(datePickerMillis: Long): Long {
        val utcCalendar = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            timeInMillis = datePickerMillis
        }
        return Calendar.getInstance().apply {
            clear()
            set(
                utcCalendar.get(Calendar.YEAR),
                utcCalendar.get(Calendar.MONTH),
                utcCalendar.get(Calendar.DAY_OF_MONTH),
                12,
                0,
                0,
            )
        }.timeInMillis
    }

    fun toDatePickerMillis(millis: Long): Long {
        val localCalendar = Calendar.getInstance().apply { timeInMillis = millis }
        return Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(
                localCalendar.get(Calendar.YEAR),
                localCalendar.get(Calendar.MONTH),
                localCalendar.get(Calendar.DAY_OF_MONTH),
                0,
                0,
                0,
            )
        }.timeInMillis
    }

    fun atLocalTime(dateMillis: Long, hour: Int, minute: Int): Long = Calendar.getInstance().apply {
        timeInMillis = dateMillis
        set(Calendar.HOUR_OF_DAY, hour)
        set(Calendar.MINUTE, minute)
        set(Calendar.SECOND, 0)
        set(Calendar.MILLISECOND, 0)
    }.timeInMillis
}

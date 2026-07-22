package com.example.myapplication.ui.util

import java.util.Calendar
import java.util.Locale
import java.util.TimeZone
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

class DateTimeUtilsTest {
    private lateinit var previousTimeZone: TimeZone
    private lateinit var previousLocale: Locale

    @Before
    fun setUp() {
        previousTimeZone = TimeZone.getDefault()
        previousLocale = Locale.getDefault()
        // Pin the environment so date arithmetic is deterministic across machines.
        TimeZone.setDefault(TimeZone.getTimeZone("America/New_York"))
        Locale.setDefault(Locale.US)
    }

    @After
    fun tearDown() {
        TimeZone.setDefault(previousTimeZone)
        Locale.setDefault(previousLocale)
    }

    private fun localMillis(
        year: Int,
        month: Int,
        day: Int,
        hour: Int = 0,
        minute: Int = 0,
        second: Int = 0,
        millis: Int = 0,
    ): Long = Calendar.getInstance().apply {
        clear()
        set(year, month, day, hour, minute, second)
        set(Calendar.MILLISECOND, millis)
    }.timeInMillis

    @Test
    fun `startOfDay zeroes the time of day`() {
        val input = localMillis(2024, Calendar.MARCH, 15, hour = 18, minute = 42, second = 7, millis = 500)

        val result = DateTimeUtils.startOfDay(input)

        assertEquals(localMillis(2024, Calendar.MARCH, 15), result)
    }

    @Test
    fun `startOfDay is idempotent`() {
        val startOfDay = localMillis(2024, Calendar.MARCH, 15)

        assertEquals(startOfDay, DateTimeUtils.startOfDay(startOfDay))
    }

    @Test
    fun `formatDueDate returns Overdue for a past day`() {
        val now = localMillis(2024, Calendar.MARCH, 15, hour = 9)
        val yesterday = localMillis(2024, Calendar.MARCH, 14, hour = 23)

        assertEquals("Overdue", DateTimeUtils.formatDueDate(yesterday, now))
    }

    @Test
    fun `formatDueDate returns Due today for a later time on the same day`() {
        val now = localMillis(2024, Calendar.MARCH, 15, hour = 9)
        val laterToday = localMillis(2024, Calendar.MARCH, 15, hour = 23, minute = 59)

        assertEquals("Due today", DateTimeUtils.formatDueDate(laterToday, now))
    }

    @Test
    fun `formatDueDate returns Due today for an earlier time on the same day`() {
        val now = localMillis(2024, Calendar.MARCH, 15, hour = 20)
        val earlierToday = localMillis(2024, Calendar.MARCH, 15, hour = 1)

        assertEquals("Due today", DateTimeUtils.formatDueDate(earlierToday, now))
    }

    @Test
    fun `formatDueDate returns Due tomorrow for the next day`() {
        val now = localMillis(2024, Calendar.MARCH, 15, hour = 9)
        val tomorrow = localMillis(2024, Calendar.MARCH, 16, hour = 1)

        assertEquals("Due tomorrow", DateTimeUtils.formatDueDate(tomorrow, now))
    }

    @Test
    fun `formatDueDate returns a formatted date beyond tomorrow`() {
        val now = localMillis(2024, Calendar.MARCH, 15, hour = 9)
        val dayAfterTomorrow = localMillis(2024, Calendar.MARCH, 17, hour = 9)

        assertEquals("Due Sun, Mar 17", DateTimeUtils.formatDueDate(dayAfterTomorrow, now))
    }

    @Test
    fun `datePickerMillisToLocalNoon maps a UTC midnight selection to local noon`() {
        // Date pickers report the selected day as UTC midnight.
        val utcMidnight = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(2024, Calendar.MARCH, 15, 0, 0, 0)
        }.timeInMillis

        val result = DateTimeUtils.datePickerMillisToLocalNoon(utcMidnight)

        assertEquals(localMillis(2024, Calendar.MARCH, 15, hour = 12), result)
    }

    @Test
    fun `toDatePickerMillis maps a local instant to UTC midnight of the same day`() {
        val localInstant = localMillis(2024, Calendar.MARCH, 15, hour = 23, minute = 30)

        val result = DateTimeUtils.toDatePickerMillis(localInstant)

        val expected = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(2024, Calendar.MARCH, 15, 0, 0, 0)
        }.timeInMillis
        assertEquals(expected, result)
    }

    @Test
    fun `datePickerMillis round trips through local noon and back`() {
        val utcMidnight = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(2024, Calendar.MARCH, 15, 0, 0, 0)
        }.timeInMillis

        val roundTrip = DateTimeUtils.toDatePickerMillis(
            DateTimeUtils.datePickerMillisToLocalNoon(utcMidnight),
        )

        assertEquals(utcMidnight, roundTrip)
    }

    @Test
    fun `atLocalTime sets the requested hour and minute and clears seconds`() {
        val day = localMillis(2024, Calendar.MARCH, 15, hour = 3, minute = 17, second = 42, millis = 999)

        val result = DateTimeUtils.atLocalTime(day, hour = 8, minute = 30)

        assertEquals(localMillis(2024, Calendar.MARCH, 15, hour = 8, minute = 30), result)
    }
}

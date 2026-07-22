package com.example.myapplication.ui.task

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import com.example.myapplication.R
import com.example.myapplication.ui.util.DateTimeUtils
import java.util.Calendar

@Composable
internal fun MetadataRow(
    title: String,
    value: String,
    icon: ImageVector,
    hasValue: Boolean,
    removeContentDescription: String,
    onClick: () -> Unit,
    onRemove: () -> Unit,
) {
    OutlinedCard(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
    ) {
        ListItem(
            headlineContent = { Text(title) },
            supportingContent = { Text(value) },
            leadingContent = {
                Icon(
                    imageVector = icon,
                    contentDescription = null,
                )
            },
            trailingContent = if (hasValue) {
                {
                    IconButton(onClick = onRemove) {
                        Icon(
                            imageVector = Icons.Filled.Close,
                            contentDescription = removeContentDescription,
                        )
                    }
                }
            } else {
                null
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun DueDatePickerDialog(
    dueDate: Long?,
    onDismiss: () -> Unit,
    onDateSelected: (Long) -> Unit,
) {
    val datePickerState = rememberDatePickerState(
        initialSelectedDateMillis = dueDate?.let(DateTimeUtils::toDatePickerMillis),
    )

    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(
                onClick = {
                    datePickerState.selectedDateMillis
                        ?.let(DateTimeUtils::datePickerMillisToLocalNoon)
                        ?.let(onDateSelected)
                        ?: onDismiss()
                },
            ) {
                Text(stringResource(R.string.done))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
        },
    ) {
        DatePicker(state = datePickerState)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun ReminderDatePickerDialog(
    reminderAt: Long?,
    onDismiss: () -> Unit,
    onDateSelected: (Long) -> Unit,
) {
    val datePickerState = rememberDatePickerState(
        initialSelectedDateMillis = reminderAt?.let(DateTimeUtils::toDatePickerMillis),
    )

    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(
                onClick = {
                    datePickerState.selectedDateMillis
                        ?.let(DateTimeUtils::datePickerMillisToLocalNoon)
                        ?.let(onDateSelected)
                        ?: onDismiss()
                },
            ) {
                Text(stringResource(R.string.next))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
        },
    ) {
        DatePicker(state = datePickerState)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun ReminderTimePickerDialog(
    initialReminderAt: Long?,
    onDismiss: () -> Unit,
    onTimeSelected: (Int, Int) -> Unit,
) {
    val initialTime = remember(initialReminderAt) {
        Calendar.getInstance().apply {
            initialReminderAt?.let { timeInMillis = it }
        }
    }
    val timePickerState = rememberTimePickerState(
        initialHour = initialTime.get(Calendar.HOUR_OF_DAY),
        initialMinute = initialTime.get(Calendar.MINUTE),
        is24Hour = android.text.format.DateFormat.is24HourFormat(LocalContext.current),
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.reminder_time_title)) },
        text = { TimePicker(state = timePickerState) },
        confirmButton = {
            TextButton(
                onClick = {
                    onTimeSelected(timePickerState.hour, timePickerState.minute)
                },
            ) {
                Text(stringResource(R.string.done))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
        },
    )
}

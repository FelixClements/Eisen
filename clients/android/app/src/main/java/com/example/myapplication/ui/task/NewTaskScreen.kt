package com.example.myapplication.ui.task

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.HelpOutline
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipAnchorPosition
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.ui.category.presentation
import com.example.myapplication.ui.theme.MyApplicationTheme
import com.example.myapplication.ui.theme.ledgerCategoryColors
import java.text.DateFormat
import java.util.Calendar
import java.util.Date
import java.util.TimeZone
import kotlinx.coroutines.launch

private const val TitleFieldTag = "new-task-title"
private const val NotesFieldTag = "new-task-notes"
private const val SaveButtonTag = "new-task-save"
private const val CategoryTagPrefix = "new-task-category-"

private val FormHorizontalGutter = 24.dp
private val FormMaxWidth = 720.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NewTaskScreen(
    defaultCategory: EisenhowerCategory,
    onSaveTask: (String, EisenhowerCategory, String?, Long?, Long?) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var title by rememberSaveable { mutableStateOf("") }
    var notes by rememberSaveable { mutableStateOf("") }
    var dueDate by rememberSaveable { mutableStateOf<Long?>(null) }
    var reminderAt by rememberSaveable { mutableStateOf<Long?>(null) }
    var reminderDate by rememberSaveable { mutableStateOf<Long?>(null) }
    var selectedCategory by rememberSaveable { mutableStateOf(defaultCategory) }
    var titleError by rememberSaveable { mutableStateOf<String?>(null) }
    var showDiscardConfirmation by rememberSaveable { mutableStateOf(false) }
    var showShortcutHelp by rememberSaveable { mutableStateOf(false) }
    var showDueDatePicker by rememberSaveable { mutableStateOf(false) }
    var showReminderDatePicker by rememberSaveable { mutableStateOf(false) }
    var showReminderTimePicker by rememberSaveable { mutableStateOf(false) }
    var titleHasFocus by remember { mutableStateOf(false) }
    var notesHasFocus by remember { mutableStateOf(false) }
    val titleFocusRequester = remember { FocusRequester() }
    val titleBringIntoViewRequester = remember { BringIntoViewRequester() }
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()
    val context = LocalContext.current
    val notificationPermissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (!granted) {
            coroutineScope.launch {
                snackbarHostState.showSnackbar(
                    "Notifications are disabled. Save the task to keep this reminder.",
                )
            }
        }
    }

    LaunchedEffect(Unit) {
        titleFocusRequester.requestFocus()
    }

    fun hasDraft(): Boolean =
        title.isNotBlank() ||
            notes.isNotEmpty() ||
            dueDate != null ||
            reminderAt != null ||
            selectedCategory != defaultCategory

    fun requestCancel() {
        if (hasDraft()) {
            showDiscardConfirmation = true
        } else {
            onCancel()
        }
    }

    fun save() {
        val trimmedTitle = title.trim()
        if (trimmedTitle.isBlank()) {
            titleError = "Task title is required."
            showDiscardConfirmation = false
            coroutineScope.launch {
                titleFocusRequester.requestFocus()
                titleBringIntoViewRequester.bringIntoView()
            }
            return
        }

        val trimmedNotes = notes.trim().takeIf { it.isNotBlank() }
        onSaveTask(trimmedTitle, selectedCategory, trimmedNotes, dueDate, reminderAt)
    }

    fun requestNotificationPermissionIfNeeded() {
        if (
            ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS,
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    val isTextFieldFocused = titleHasFocus || notesHasFocus
    val isOverlayOpen =
        showDiscardConfirmation ||
            showShortcutHelp ||
            showDueDatePicker ||
            showReminderDatePicker ||
            showReminderTimePicker

    BackHandler(onBack = ::requestCancel)

    Scaffold(
        modifier = modifier
            .fillMaxSize()
            .onPreviewKeyEvent { event ->
                if (isOverlayOpen) {
                    return@onPreviewKeyEvent false
                }

                if (event.type != KeyEventType.KeyDown) {
                    return@onPreviewKeyEvent false
                }

                event.categoryShortcut()?.let { category ->
                    selectedCategory = category
                    return@onPreviewKeyEvent true
                }

                if (isTextFieldFocused) {
                    return@onPreviewKeyEvent false
                }

                if (event.key == Key.Escape) {
                    requestCancel()
                    return@onPreviewKeyEvent true
                }

                false
            },
        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
        topBar = {
            TopAppBar(
                title = { Text("New Task") },
                actions = {
                    TooltipBox(
                        positionProvider = TooltipDefaults.rememberTooltipPositionProvider(
                            TooltipAnchorPosition.Above,
                        ),
                        tooltip = { PlainTooltip { Text("Composer keyboard shortcuts") } },
                        state = rememberTooltipState(),
                    ) {
                        IconButton(onClick = { showShortcutHelp = true }) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.HelpOutline,
                                contentDescription = "Composer keyboard shortcuts",
                            )
                        }
                    }
                    TextButton(onClick = ::requestCancel) {
                        Text("Cancel")
                    }
                },
            )
        },
        bottomBar = {
            // The bottom app bar owns navigation and IME insets. Its measured height is
            // included in Scaffold's innerPadding, so the scroll content clears it once.
            BottomAppBar(
                windowInsets = WindowInsets.navigationBars.union(WindowInsets.ime),
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Box(
                        modifier = Modifier
                            .widthIn(max = FormMaxWidth)
                            .fillMaxWidth()
                            .padding(horizontal = FormHorizontalGutter),
                    ) {
                        Button(
                            onClick = ::save,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(56.dp)
                                .testTag(SaveButtonTag),
                        ) {
                            Text("Save")
                        }
                    }
                }
            }
        },
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
        ) {
            Column(
                modifier = Modifier
                    .widthIn(max = FormMaxWidth)
                    .fillMaxWidth()
                    .align(Alignment.TopCenter)
                    .padding(horizontal = FormHorizontalGutter, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(20.dp),
            ) {
            OutlinedTextField(
                value = title,
                onValueChange = { newTitle ->
                    title = newTitle
                    titleError = null
                    showDiscardConfirmation = false
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .focusRequester(titleFocusRequester)
                    .bringIntoViewRequester(titleBringIntoViewRequester)
                    .onFocusChanged { titleHasFocus = it.isFocused }
                    .onPreviewKeyEvent { event ->
                        if (event.type == KeyEventType.KeyDown && event.key == Key.Enter) {
                            save()
                            true
                        } else {
                            false
                        }
                    }
                    .testTag(TitleFieldTag),
                label = { Text("Task title") },
                supportingText = { Text(titleError ?: "Required") },
                textStyle = MaterialTheme.typography.headlineSmall,
                singleLine = true,
                isError = titleError != null,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { save() }),
            )

            Text(
                text = "Category",
                modifier = Modifier.semantics { heading() },
                style = MaterialTheme.typography.titleMedium,
            )

            CategoryGrid(
                selectedCategory = selectedCategory,
                onCategorySelected = {
                    selectedCategory = it
                    showDiscardConfirmation = false
                },
            )

            DetailsArea(
                notes = notes,
                dueDate = dueDate,
                reminderAt = reminderAt,
                onNotesChange = { newNotes ->
                    notes = newNotes
                    showDiscardConfirmation = false
                },
                onDueDateClick = { showDueDatePicker = true },
                onDueDateRemove = {
                    dueDate = null
                    showDiscardConfirmation = false
                },
                onReminderClick = { showReminderDatePicker = true },
                onReminderRemove = {
                    reminderAt = null
                    reminderDate = null
                    showDiscardConfirmation = false
                },
                onNotesFocusChanged = { notesHasFocus = it },
            )
            }
        }
    }

    if (showDiscardConfirmation) {
        DiscardConfirmationDialog(
            onKeepEditing = { showDiscardConfirmation = false },
            onDiscard = onCancel,
        )
    }

    if (showShortcutHelp) {
        ComposerShortcutHelpDialog(onDismiss = { showShortcutHelp = false })
    }

    if (showDueDatePicker) {
        DueDatePickerDialog(
            dueDate = dueDate,
            onDismiss = { showDueDatePicker = false },
            onDateSelected = { selectedDate ->
                dueDate = selectedDate
                showDueDatePicker = false
                showDiscardConfirmation = false
            },
        )
    }

    if (showReminderDatePicker) {
        ReminderDatePickerDialog(
            reminderAt = reminderAt,
            onDismiss = { showReminderDatePicker = false },
            onDateSelected = { selectedDate ->
                reminderDate = selectedDate
                showReminderDatePicker = false
                showReminderTimePicker = true
            },
        )
    }

    if (showReminderTimePicker) {
        ReminderTimePickerDialog(
            initialReminderAt = reminderAt,
            onDismiss = { showReminderTimePicker = false },
            onTimeSelected = { hour, minute ->
                reminderDate?.let { selectedDate ->
                    reminderAt = selectedDate.atLocalTime(hour, minute)
                    showDiscardConfirmation = false
                    requestNotificationPermissionIfNeeded()
                }
                showReminderTimePicker = false
            },
        )
    }
}

@Composable
private fun DetailsArea(
    notes: String,
    dueDate: Long?,
    reminderAt: Long?,
    onNotesChange: (String) -> Unit,
    onDueDateClick: () -> Unit,
    onDueDateRemove: () -> Unit,
    onReminderClick: () -> Unit,
    onReminderRemove: () -> Unit,
    onNotesFocusChanged: (Boolean) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            text = "Details",
            modifier = Modifier.semantics { heading() },
            style = MaterialTheme.typography.titleMedium,
        )
        OutlinedTextField(
            value = notes,
            onValueChange = onNotesChange,
            modifier = Modifier
                .fillMaxWidth()
                .onFocusChanged { onNotesFocusChanged(it.isFocused) }
                .testTag(NotesFieldTag),
            label = { Text("Notes") },
            supportingText = { Text("Optional") },
            minLines = 6,
            maxLines = 6,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Default),
        )
        MetadataRow(
            title = "Due date",
            value = dueDate?.let(::formatDueDate) ?: "Add date",
            icon = Icons.Filled.Event,
            hasValue = dueDate != null,
            removeContentDescription = "Remove due date",
            onClick = onDueDateClick,
            onRemove = onDueDateRemove,
        )
        MetadataRow(
            title = "Reminder",
            value = reminderAt?.let(::formatReminderAt) ?: "Add reminder",
            icon = Icons.Filled.Notifications,
            hasValue = reminderAt != null,
            removeContentDescription = "Remove reminder",
            onClick = onReminderClick,
            onRemove = onReminderRemove,
        )
    }
}

@Composable
private fun MetadataRow(
    title: String,
    value: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
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
private fun DueDatePickerDialog(
    dueDate: Long?,
    onDismiss: () -> Unit,
    onDateSelected: (Long) -> Unit,
) {
    val datePickerState = rememberDatePickerState(
        initialSelectedDateMillis = dueDate?.toDatePickerMillis(),
    )

    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(
                onClick = {
                    datePickerState.selectedDateMillis
                        ?.let(::datePickerMillisToLocalNoon)
                        ?.let(onDateSelected)
                        ?: onDismiss()
                },
            ) {
                Text("Done")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    ) {
        DatePicker(state = datePickerState)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReminderDatePickerDialog(
    reminderAt: Long?,
    onDismiss: () -> Unit,
    onDateSelected: (Long) -> Unit,
) {
    val datePickerState = rememberDatePickerState(
        initialSelectedDateMillis = reminderAt?.toDatePickerMillis(),
    )

    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(
                onClick = {
                    datePickerState.selectedDateMillis
                        ?.let(::datePickerMillisToLocalNoon)
                        ?.let(onDateSelected)
                        ?: onDismiss()
                },
            ) {
                Text("Next")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    ) {
        DatePicker(state = datePickerState)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReminderTimePickerDialog(
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
        title = { Text("Reminder time") },
        text = { TimePicker(state = timePickerState) },
        confirmButton = {
            TextButton(
                onClick = {
                    onTimeSelected(timePickerState.hour, timePickerState.minute)
                },
            ) {
                Text("Done")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun CategoryGrid(
    selectedCategory: EisenhowerCategory,
    onCategorySelected: (EisenhowerCategory) -> Unit,
) {
    val categories = EisenhowerCategory.entries
    CategorySegmentedRadioRow(
        categories = categories,
        selectedCategory = selectedCategory,
        onCategorySelected = onCategorySelected,
    )
}

@Composable
private fun CategorySegmentedRadioRow(
    categories: List<EisenhowerCategory>,
    selectedCategory: EisenhowerCategory,
    onCategorySelected: (EisenhowerCategory) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .selectableGroup(),
    ) {
        categories.forEachIndexed { index, category ->
            CategorySegmentedRadioButton(
                category = category,
                selected = category == selectedCategory,
                index = index,
                count = categories.size,
                modifier = Modifier.weight(1f),
                onClick = { onCategorySelected(category) },
            )
        }
    }
}

@Composable
private fun CategorySegmentedRadioButton(
    category: EisenhowerCategory,
    selected: Boolean,
    index: Int,
    count: Int,
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val presentation = category.presentation()
    val categoryColors = category.ledgerCategoryColors()
    var isFocused by remember { mutableStateOf(false) }
    val border = when {
        isFocused -> BorderStroke(2.dp, MaterialTheme.colorScheme.primary)
        selected -> BorderStroke(2.dp, categoryColors.outline)
        else -> BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    }
    OutlinedCard(
        modifier = modifier
            .height(64.dp)
            .testTag(CategoryTagPrefix + category.name)
            .semantics { contentDescription = presentation.label }
            .onFocusChanged { isFocused = it.isFocused }
            .selectable(
                selected = selected,
                onClick = onClick,
                role = Role.RadioButton,
            ),
        shape = RectangleShape,
        colors = if (selected) {
            CardDefaults.outlinedCardColors(
                containerColor = categoryColors.container,
                contentColor = categoryColors.onContainer,
            )
        } else {
            CardDefaults.outlinedCardColors()
        },
        border = border,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .height(64.dp)
                .padding(horizontal = 4.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Icon(
                imageVector = presentation.icon,
                modifier = Modifier.size(20.dp),
                contentDescription = null,
                tint = if (selected) {
                    categoryColors.onContainer
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
            Text(
                text = presentation.selectorLabel,
                style = MaterialTheme.typography.labelSmall,
                maxLines = 1,
                softWrap = false,
            )
        }
    }
}

@Composable
private fun DiscardConfirmationDialog(
    onKeepEditing: () -> Unit,
    onDiscard: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onKeepEditing,
        title = { Text("Discard draft?") },
        text = { Text("Your unsaved task details will be lost.") },
        confirmButton = {
            TextButton(
                onClick = onDiscard,
                colors = androidx.compose.material3.ButtonDefaults.textButtonColors(
                    contentColor = MaterialTheme.colorScheme.error,
                ),
            ) {
                Text("Discard")
            }
        },
        dismissButton = {
            TextButton(onClick = onKeepEditing) {
                Text("Keep editing")
            }
        },
    )
}

@Composable
private fun ComposerShortcutHelpDialog(onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("New Task shortcuts") },
        text = {
            Column {
                ComposerShortcutHelpRow(keys = "Enter (title)", action = "Save task")
                ComposerShortcutHelpRow(keys = "Back / Cancel", action = "Cancel task")
                EisenhowerCategory.entries.forEach { category ->
                    val presentation = category.presentation()
                    ComposerShortcutHelpRow(
                        keys = "Alt + ${presentation.shortcutLabel}",
                        action = "Select ${presentation.label}",
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text("Done") }
        },
    )
}

@Composable
private fun ComposerShortcutHelpRow(
    keys: String,
    action: String,
) {
    ListItem(
        headlineContent = { Text(action) },
        supportingContent = { Text(keys) },
    )
}

private fun androidx.compose.ui.input.key.KeyEvent.categoryShortcut(): EisenhowerCategory? {
    if (!isAltPressed) return null

    return when (key) {
        Key.Q -> EisenhowerCategory.DO_NOW
        Key.W -> EisenhowerCategory.SCHEDULE
        Key.E -> EisenhowerCategory.DELEGATE_WAITING
        Key.R -> EisenhowerCategory.ELIMINATE_LATER
        else -> null
    }
}

private fun datePickerMillisToLocalNoon(datePickerMillis: Long): Long {
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

private fun Long.toDatePickerMillis(): Long {
    val localCalendar = Calendar.getInstance().apply { timeInMillis = this@toDatePickerMillis }
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

private fun formatDueDate(dueDate: Long): String =
    DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date(dueDate))

private fun Long.atLocalTime(hour: Int, minute: Int): Long = Calendar.getInstance().apply {
    timeInMillis = this@atLocalTime
    set(Calendar.HOUR_OF_DAY, hour)
    set(Calendar.MINUTE, minute)
    set(Calendar.SECOND, 0)
    set(Calendar.MILLISECOND, 0)
}.timeInMillis

private fun formatReminderAt(reminderAt: Long): String =
    DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT).format(Date(reminderAt))

@Preview(name = "New task (573dp)", widthDp = 573, heightDp = 800)
@Composable
private fun NewTask573Preview() {
    MyApplicationTheme(darkTheme = true, dynamicColor = false) {
        NewTaskScreen(
            defaultCategory = EisenhowerCategory.SCHEDULE,
            onSaveTask = { _, _, _, _, _ -> },
            onCancel = {},
        )
    }
}

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
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.example.myapplication.R
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.ui.category.presentation
import com.example.myapplication.ui.theme.MyApplicationTheme
import com.example.myapplication.ui.theme.ledgerCategoryColors
import com.example.myapplication.ui.util.DateTimeUtils
import java.util.Calendar
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
    onSaveTask: (String, EisenhowerCategory, String?, Long?, Long?, String?, onDone: (Result<*>) -> Unit) -> Unit,
    onTaskSaved: () -> Unit = {},
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var title by rememberSaveable { mutableStateOf("") }
    var notes by rememberSaveable { mutableStateOf("") }
    var taskCategory by rememberSaveable { mutableStateOf("") }
    var dueDate by rememberSaveable { mutableStateOf<Long?>(null) }
    var reminderAt by rememberSaveable { mutableStateOf<Long?>(null) }
    var reminderDate by rememberSaveable { mutableStateOf<Long?>(null) }
    var selectedCategory by rememberSaveable { mutableStateOf(defaultCategory) }
    var titleError by rememberSaveable { mutableStateOf<String?>(null) }
    var saveError by rememberSaveable { mutableStateOf<String?>(null) }
    var isSaving by rememberSaveable { mutableStateOf(false) }
    var showDiscardConfirmation by rememberSaveable { mutableStateOf(false) }
    var showShortcutHelp by rememberSaveable { mutableStateOf(false) }
    var showOverflowMenu by rememberSaveable { mutableStateOf(false) }
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
        !isSaving &&
            (title.isNotBlank() ||
                notes.isNotEmpty() ||
                taskCategory.isNotEmpty() ||
                dueDate != null ||
                reminderAt != null ||
                selectedCategory != defaultCategory)

    fun requestCancel() {
        if (isSaving) return // don't interrupt save
        if (hasDraft()) {
            showDiscardConfirmation = true
        } else {
            onCancel()
        }
    }

    val titleRequiredMsg = stringResource(R.string.title_required)
    val reminderPastMsg = stringResource(R.string.reminder_past_warning)

    fun save() {
        if (isSaving) return // prevent duplicate taps
        val trimmedTitle = title.trim()
        if (trimmedTitle.isBlank()) {
            titleError = titleRequiredMsg
            showDiscardConfirmation = false
            coroutineScope.launch {
                titleFocusRequester.requestFocus()
                titleBringIntoViewRequester.bringIntoView()
            }
            return
        }

        saveError = null
        val now = System.currentTimeMillis()
        if (reminderAt != null && reminderAt!! < now) {
            coroutineScope.launch {
                snackbarHostState.showSnackbar(reminderPastMsg)
            }
        }
        
        isSaving = true
        val trimmedNotes = notes.trim().takeIf { it.isNotBlank() }
        val trimmedCategory = taskCategory.trim().takeIf { it.isNotBlank() }
        onSaveTask(trimmedTitle, selectedCategory, trimmedNotes, dueDate, reminderAt, trimmedCategory) { result ->
            isSaving = false
            if (result.isSuccess) {
                onTaskSaved()
            } else {
                saveError = "Could not save task. Check your connection and try again."
                coroutineScope.launch {
                    snackbarHostState.showSnackbar(
                        message = "Failed to save task",
                        actionLabel = "Retry",
                    ).let { action ->
                        if (action == SnackbarResult.ActionPerformed) {
                            save()
                        }
                    }
                }
            }
        }
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
                title = { Text(stringResource(R.string.new_task_title)) },
                actions = {
                    Box {
                        IconButton(onClick = { showOverflowMenu = true }) {
                            Icon(
                                imageVector = Icons.Filled.MoreVert,
                                contentDescription = "More actions",
                            )
                        }
                        DropdownMenu(
                            expanded = showOverflowMenu,
                            onDismissRequest = { showOverflowMenu = false },
                        ) {
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.keyboard_shortcuts_title)) },
                                leadingIcon = {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Filled.HelpOutline,
                                        contentDescription = null,
                                    )
                                },
                                onClick = {
                                    showOverflowMenu = false
                                    showShortcutHelp = true
                                },
                            )
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.cancel)) },
                                onClick = {
                                    showOverflowMenu = false
                                    requestCancel()
                                },
                            )
                        }
                    }
                },
            )
        },
        bottomBar = {
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
                            enabled = !isSaving,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(56.dp)
                                .testTag(SaveButtonTag),
                        ) {
                            Text(if (isSaving) "Saving…" else stringResource(R.string.done))
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
                label = { Text(stringResource(R.string.title_label)) },
                supportingText = { Text(titleError ?: stringResource(R.string.title_hint)) },
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
                taskCategory = taskCategory,
                dueDate = dueDate,
                reminderAt = reminderAt,
                onNotesChange = { newNotes ->
                    notes = newNotes
                    showDiscardConfirmation = false
                },
                onTaskCategoryChange = { newTaskCategory ->
                    taskCategory = newTaskCategory
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
                    reminderAt = DateTimeUtils.atLocalTime(selectedDate, hour, minute)
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
    taskCategory: String,
    dueDate: Long?,
    reminderAt: Long?,
    onNotesChange: (String) -> Unit,
    onTaskCategoryChange: (String) -> Unit,
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
            value = taskCategory,
            onValueChange = onTaskCategoryChange,
            modifier = Modifier.fillMaxWidth(),
            label = { Text(stringResource(R.string.category_label)) },
            supportingText = { Text(stringResource(R.string.category_hint)) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
        )
        OutlinedTextField(
            value = notes,
            onValueChange = onNotesChange,
            modifier = Modifier
                .fillMaxWidth()
                .onFocusChanged { onNotesFocusChanged(it.isFocused) }
                .testTag(NotesFieldTag),
            label = { Text(stringResource(R.string.notes_label)) },
            supportingText = { Text(stringResource(R.string.notes_hint)) },
            minLines = 6,
            maxLines = 6,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Default),
        )
        MetadataRow(
            title = stringResource(R.string.due_date_label),
            value = dueDate?.let { DateTimeUtils.formatDueDate(it, System.currentTimeMillis()) } ?: stringResource(R.string.add_date),
            icon = Icons.Filled.Event,
            hasValue = dueDate != null,
            removeContentDescription = stringResource(R.string.remove_due_date),
            onClick = onDueDateClick,
            onRemove = onDueDateRemove,
        )
        MetadataRow(
            title = stringResource(R.string.reminder_label),
            value = reminderAt?.let { DateTimeUtils.formatReminderAt(it, LocalContext.current) } ?: stringResource(R.string.add_reminder),
            icon = Icons.Filled.Notifications,
            hasValue = reminderAt != null,
            removeContentDescription = stringResource(R.string.remove_reminder),
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
private fun ReminderDatePickerDialog(
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
        title = { Text(stringResource(R.string.discard_draft_title)) },
        text = { Text(stringResource(R.string.discard_draft_message)) },
        confirmButton = {
            TextButton(
                onClick = onDiscard,
                colors = androidx.compose.material3.ButtonDefaults.textButtonColors(
                    contentColor = MaterialTheme.colorScheme.error,
                ),
            ) {
                Text(stringResource(R.string.discard))
            }
        },
        dismissButton = {
            TextButton(onClick = onKeepEditing) {
                Text(stringResource(R.string.keep_editing))
            }
        },
    )
}

@Composable
private fun ComposerShortcutHelpDialog(onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.new_task_shortcuts)) },
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
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.done)) }
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

@Preview(name = "New task (573dp)", widthDp = 573, heightDp = 800)
@Composable
private fun NewTask573Preview() {
    MyApplicationTheme(darkTheme = true, dynamicColor = false) {
        NewTaskScreen(
            defaultCategory = EisenhowerCategory.SCHEDULE,
            onSaveTask = { _, _, _, _, _, _, _ -> },
            onCancel = {},
        )
    }
}

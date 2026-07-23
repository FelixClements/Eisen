package com.example.myapplication.ui.task

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Archive
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.PushPin
import androidx.compose.material.icons.filled.Unarchive
import androidx.compose.material3.CardDefaults
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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.unit.dp
import com.example.myapplication.R
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.ui.category.presentation
import com.example.myapplication.ui.components.ScreenTopBar
import com.example.myapplication.ui.components.TopBarHeight
import com.example.myapplication.ui.theme.ledgerCategoryColors
import com.example.myapplication.ui.util.DateTimeUtils
import java.util.TimeZone
import kotlinx.coroutines.launch

@Composable
fun TaskDetailScreen(
    viewModel: TaskDetailViewModel,
    onBack: () -> Unit,
    onOpenKeyboardShortcuts: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val task by viewModel.task.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()

    val reminderPastMsg = stringResource(R.string.reminder_past_warning)

    LaunchedEffect(viewModel) {
        viewModel.events.collect { event ->
            when (event) {
                is TaskDetailUiEvent.OperationFailed -> snackbarHostState.showSnackbar(event.message)
            }
        }
    }

    task?.let { currentTask ->
        var title by remember(currentTask.id) { mutableStateOf(currentTask.title) }
        var taskCategory by remember(currentTask.id) { mutableStateOf(currentTask.category ?: "") }
        var description by remember(currentTask.id) { mutableStateOf(currentTask.description ?: "") }
        var showDueDatePicker by remember { mutableStateOf(false) }
        var showReminderDatePicker by remember { mutableStateOf(false) }
        var showReminderTimePicker by remember { mutableStateOf(false) }
        var reminderDate by remember { mutableStateOf<Long?>(null) }
        var titleHasFocus by remember { mutableStateOf(false) }
        var notesHasFocus by remember { mutableStateOf(false) }
        var categoryHasFocus by remember { mutableStateOf(false) }

        val isTextFieldFocused = titleHasFocus || notesHasFocus || categoryHasFocus

        Scaffold(
            modifier = modifier
                .fillMaxSize()
                .onPreviewKeyEvent { event ->
                    if (isTextFieldFocused) return@onPreviewKeyEvent false
                    if (event.type == KeyEventType.KeyDown) {
                        when (event.key) {
                            Key.M -> {
                                // Since detail doesn't have a drawer button, we skip this or map it to something else
                                // But for consistency, let's just ignore M in detail for now or map it if it makes sense
                                false
                            }
                            Key.Slash -> {
                                if (event.isShiftPressed) {
                                    onOpenKeyboardShortcuts()
                                    true
                                } else false
                            }
                            else -> false
                        }
                    } else false
                },
            contentWindowInsets = WindowInsets.navigationBars,
            snackbarHost = { SnackbarHost(snackbarHostState) }
        ) { innerPadding ->
            val topBarPadding = WindowInsets.statusBars.asPaddingValues().calculateTopPadding() + TopBarHeight + 16.dp
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
                contentAlignment = Alignment.TopCenter
            ) {
                Column(
                    modifier = Modifier
                        .widthIn(max = 720.dp)
                        .fillMaxWidth()
                        .padding(16.dp)
                        .padding(top = topBarPadding)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    OutlinedTextField(
                        value = title,
                        onValueChange = {
                            title = it
                            if (it.isNotBlank()) {
                                viewModel.updateTask(currentTask.copy(title = it))
                            }
                        },
                        label = { Text(stringResource(R.string.title_label)) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .onFocusChanged { titleHasFocus = it.isFocused },
                        textStyle = MaterialTheme.typography.titleLarge
                    )

                    OutlinedTextField(
                        value = taskCategory,
                        onValueChange = {
                            taskCategory = it
                            viewModel.updateTask(currentTask.copy(category = it.takeIf { it.isNotBlank() }))
                        },
                        label = { Text(stringResource(R.string.category_label)) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .onFocusChanged { categoryHasFocus = it.isFocused },
                        singleLine = true
                    )

                    OutlinedTextField(
                        value = description,
                        onValueChange = {
                            description = it
                            viewModel.updateTask(currentTask.copy(description = it.takeIf { it.isNotBlank() }))
                        },
                        label = { Text(stringResource(R.string.notes_label)) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .onFocusChanged { notesHasFocus = it.isFocused },
                        minLines = 5
                    )

                    Text(
                        text = "Eisenhower Category",
                        style = MaterialTheme.typography.titleMedium
                    )

                    val currentEisenhower = EisenhowerCategory.entries.find {
                        it.isImportant == currentTask.isImportant && it.isUrgent == currentTask.isUrgent
                    } ?: EisenhowerCategory.DO_NOW

                    CategorySegmentedRadioRow(
                        selectedCategory = currentEisenhower,
                        onCategorySelected = { category ->
                            viewModel.updateTask(
                                currentTask.copy(
                                    isImportant = category.isImportant,
                                    isUrgent = category.isUrgent
                                )
                            )
                        }
                    )

                    MetadataRow(
                        title = stringResource(R.string.due_date_label),
                        value = currentTask.dueDate?.let { DateTimeUtils.formatDueDate(it, System.currentTimeMillis()) } ?: stringResource(R.string.add_date),
                        icon = Icons.Filled.Event,
                        hasValue = currentTask.dueDate != null,
                        removeContentDescription = stringResource(R.string.remove_due_date),
                        onClick = { showDueDatePicker = true },
                        onRemove = {
                            viewModel.updateTask(currentTask.copy(dueDate = null))
                        },
                    )

                    MetadataRow(
                        title = stringResource(R.string.reminder_label),
                        value = currentTask.reminderAt?.let { DateTimeUtils.formatReminderAt(it, LocalContext.current) } ?: stringResource(R.string.add_reminder),
                        icon = Icons.Filled.Notifications,
                        hasValue = currentTask.reminderAt != null,
                        removeContentDescription = stringResource(R.string.remove_reminder),
                        onClick = { showReminderDatePicker = true },
                        onRemove = {
                            viewModel.updateTask(currentTask.copy(reminderAt = null))
                        },
                    )

                    ListItem(
                        headlineContent = { Text(stringResource(R.string.status_label)) },
                        supportingContent = {
                            Text(
                                when {
                                    currentTask.isArchived -> stringResource(R.string.status_archived)
                                    currentTask.isCompleted -> stringResource(R.string.status_completed)
                                    else -> stringResource(R.string.status_active)
                                }
                            )
                        },
                        trailingContent = {
                            IconButton(onClick = {
                                viewModel.updateTask(currentTask.copy(isCompleted = !currentTask.isCompleted))
                            }) {
                                Icon(
                                    imageVector = if (currentTask.isCompleted) Icons.Filled.Check else Icons.Filled.Close,
                                    contentDescription = if (currentTask.isCompleted) stringResource(R.string.incomplete_task, currentTask.title) else stringResource(R.string.complete_task, currentTask.title)
                                )
                            }
                        }
                    )
                }

                ScreenTopBar(
                    title = { Text(stringResource(R.string.task_detail_title)) },
                    navigationIcon = {
                        IconButton(onClick = onBack) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.back_button))
                        }
                    },
                    actions = {
                        IconButton(onClick = {
                            viewModel.updateTask(currentTask.copy(isPinned = !currentTask.isPinned))
                        }) {
                            Icon(
                                imageVector = Icons.Filled.PushPin,
                                contentDescription = if (currentTask.isPinned) stringResource(R.string.unpin_task) else stringResource(R.string.pin_task),
                                tint = if (currentTask.isPinned) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        if (!currentTask.isArchived && !currentTask.isCompleted) {
                            IconButton(onClick = {
                                viewModel.updateTask(currentTask.copy(isArchived = true))
                            }) {
                                Icon(Icons.Filled.Archive, contentDescription = stringResource(R.string.archive_task, currentTask.title))
                            }
                        } else if (currentTask.isArchived) {
                            IconButton(onClick = {
                                viewModel.updateTask(currentTask.copy(isArchived = false))
                            }) {
                                Icon(Icons.Filled.Unarchive, contentDescription = stringResource(R.string.unarchive_task, currentTask.title))
                            }
                        }
                    },
                    modifier = Modifier.align(Alignment.TopCenter),
                )
            }
        }

        if (showDueDatePicker) {
            DueDatePickerDialog(
                dueDate = currentTask.dueDate,
                onDismiss = { showDueDatePicker = false },
                onDateSelected = { selectedDate ->
                    viewModel.updateTask(currentTask.copy(dueDate = selectedDate))
                    showDueDatePicker = false
                },
            )
        }

        if (showReminderDatePicker) {
            ReminderDatePickerDialog(
                reminderAt = currentTask.reminderAt,
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
                initialReminderAt = currentTask.reminderAt,
                onDismiss = { showReminderTimePicker = false },
                onTimeSelected = { hour, minute ->
                    reminderDate?.let { date ->
                        val newReminderAt = DateTimeUtils.atLocalTime(date, hour, minute)
                        if (newReminderAt < System.currentTimeMillis()) {
                            coroutineScope.launch {
                                snackbarHostState.showSnackbar(reminderPastMsg)
                            }
                        }
                        viewModel.updateTask(currentTask.copy(reminderAt = newReminderAt))
                    }
                    showReminderTimePicker = false
                },
            )
        }
    } ?: run {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text("Task not found")
        }
    }
}

@Composable
private fun CategorySegmentedRadioRow(
    selectedCategory: EisenhowerCategory,
    onCategorySelected: (EisenhowerCategory) -> Unit,
) {
    val categories = EisenhowerCategory.entries
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .selectableGroup(),
    ) {
        categories.forEach { category ->
            val presentation = category.presentation()
            val categoryColors = category.ledgerCategoryColors()
            val selected = category == selectedCategory
            
            OutlinedCard(
                modifier = Modifier
                    .weight(1f)
                    .height(64.dp)
                    .selectable(
                        selected = selected,
                        onClick = { onCategorySelected(category) },
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
            ) {
                Column(
                    modifier = Modifier.fillMaxSize(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center
                ) {
                    Icon(
                        imageVector = presentation.icon,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp)
                    )
                    Text(
                        text = presentation.selectorLabel,
                        style = MaterialTheme.typography.labelSmall
                    )
                }
            }
        }
    }
}

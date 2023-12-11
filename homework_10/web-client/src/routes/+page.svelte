<script>
	import { onMount, onDestroy, afterUpdate } from 'svelte';

	let messages = [];
	let messageIds = new Set();
	let messagesContainer;
	let filter = '';

	async function fetchMessages() {
		const response = await fetch(`http://localhost:11112/messages?username=${filter}`);
		if (response.ok) {
			let newMessages = await response.json();
			newMessages = newMessages.reverse();
			newMessages.forEach((message) => {
				if (!messageIds.has(message.id)) {
					messageIds.add(message.id);
					messages = [...messages, message];
				}
			});
		} else {
			console.error('Failed to fetch messages');
		}
	}

	let interval;
	onMount(() => {
		fetchMessages();
		interval = setInterval(fetchMessages, 2000);
	});

	onDestroy(() => {
		clearInterval(interval);
	});

	afterUpdate(() => {
		if (messagesContainer) {
			scrollToBottom();
		}
	});

	function scrollToBottom() {
		messagesContainer.scrollTop = messagesContainer.scrollHeight;
	}

	function formatTime(timestamp) {
		return new Date(timestamp * 1000).toLocaleTimeString('en-US', {
			hour: '2-digit',
			minute: '2-digit',
			hour12: false
		});
	}

  function filterMsg() {
    messages = [];
    messageIds = new Set();
  }
</script>

<h1>Welcome to Chat client!</h1>

<input type="text" bind:value={filter} on:input={filterMsg} placeholder="Filter by username" />
<div class="messages-container">

	<div class="messages-list" bind:this={messagesContainer}>
		{#each messages as message}
			<div class="message-container">
				<div class="message">
					<strong>{message.username}</strong>: {message.text}
					<small>{formatTime(message.timestamp)}</small>
				</div>
			</div>
		{/each}
	</div>
</div>

<style>
	.messages-container {
		display: flex;
		justify-content: center;
		align-items: flex-start;
		height: 500px;
		padding: 20px;
		background-color: black;
		color: white;
		font-family: Helvetica, Arial, sans-serif;
	}

	.messages-list {
		width: 500px;
		max-height: 500px;
		overflow-y: auto;
		border: 1px solid #ccc;
		border-radius: 10px;
		padding: 10px;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.message-container {
		margin-bottom: 5px;
		max-width: 100%;
	}
	.message {
		display: block;
		margin-bottom: 10px;
		padding: 8px 12px;
		background-color: #333;
		color: white;
		font-family: Helvetica, Arial, sans-serif;
		word-wrap: break-word;
		max-width: 100%;
		border-radius: 4px;
		display: inline-block;
	}
</style>

<script>
	import { onMount, onDestroy, afterUpdate } from 'svelte';

	let messages = [];
	let messageIds = new Set();
	let messagesContainer;
	let filter = '';

	let users = [];
	let userIds = new Set();

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

	async function fetchUsers() {
		const response = await fetch(`http://localhost:11112/users`);
		if (response.ok) {
			let newUsers = await response.json();
			newUsers.forEach((user) => {
				if (!userIds.has(user.id)) {
					userIds.add(user.id);
					users = [...users, user];
				}
			});
		}
	}

	async function fetchUsersAndMessages() {
		return Promise.all([fetchUsers(), fetchMessages()]);
	}

	async function deleteUser(id) {
		const response = await fetch(`http://localhost:11112/user/${id}`, {
			method: 'DELETE'
		});
		if (response.ok) {
			clearMessages();
			users = users.filter((user) => user.id !== id);
			userIds.delete(id);
		}
	}

	let interval;
	onMount(() => {
		fetchMessages();
		interval = setInterval(fetchUsersAndMessages, 2000);
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

	function clearMessages() {
		messages = [];
		messageIds = new Set();
	}
</script>

<div class="main-container">
    <h1>Welcome to Chat client!</h1>
	Filter messages:
	<input type="text" bind:value={filter} on:input={clearMessages} placeholder="username" />
    <div class="content-container">
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

        <div class="users-container">
            <div class="users-list">
				{#each users as user}
					<div class="user-container">
						<div class="user">
							<strong>{user.username}</strong>
							<button
								class="button"
								on:click={() => {
									deleteUser(user.id);
								}}>Delete</button
							>
						</div>
					</div>
				{/each}
			</div>
        </div>
    </div>
</div>



<style>

	.main-container {
		text-align: center;
		background-color: black;
		color: white;
		font-family: Helvetica, Arial, sans-serif;
	}

	.content-container {
		display: flex;
		justify-content: center;
		gap: 20px;
		padding: 20px;
	}

	.messages-container, .users-container {
		flex: 1; 
		max-width: 500px;
	}

	.messages-container {
		text-align: left;
		display: flex;
		justify-content: center;
		align-items: flex-start;
		height: 500px;
		padding: 20px;
		
	}

	.messages-list {
		width: 500px;
		max-height: 500px;
		overflow-y: auto;
		border: 1px solid #ccc;
		border-radius: 10px;
		padding: 10px;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		min-height: 500px;
	}

	.message-container {
		margin-bottom: 5px;
		max-width: 100%;
	}

	.message {
		display: block;
		margin-bottom: 10px;
		padding: 8px 12px;
	
		word-wrap: break-word;
		max-width: 100%;
		border-radius: 4px;
		display: inline-block;
	}

	.button {
		margin-left: 10px;
		background-color: red;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	.users-container {
		text-align: left;
		padding: 10px;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		overflow-y: auto; 
	}
</style>

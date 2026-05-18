# Discord.js — Quick Reference (v14)

## Client Setup
```ts
import { Client, Events, GatewayIntentBits, Partials } from 'discord.js';

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.GuildMembers,
    GatewayIntentBits.MessageContent,
    GatewayIntentBits.GuildVoiceStates,
  ],
  partials: [Partials.Message, Partials.Channel],
});

client.once(Events.ClientReady, (c) => console.log(`Ready as ${c.user.tag}`));
client.on(Events.InteractionCreate, async (interaction) => { ... });
client.login(process.env.DISCORD_TOKEN);
```

## Slash Commands
```ts
import { SlashCommandBuilder, PermissionFlagsBits } from 'discord.js';

const command = new SlashCommandBuilder()
  .setName('ban')
  .setDescription('Ban a member')
  .addUserOption(opt => opt.setName('target').setDescription('User').setRequired(true))
  .addStringOption(opt => opt.setName('reason').setDescription('Why'))
  .setDefaultMemberPermissions(PermissionFlagsBits.BanMembers);

// Handle
async execute(interaction) {
  const target = interaction.options.getUser('target');
  const reason = interaction.options.getString('reason') ?? 'No reason';
  await interaction.reply(`Banned ${target.tag}: ${reason}`);
}
```

## Embeds
```ts
import { EmbedBuilder, Colors } from 'discord.js';

const embed = new EmbedBuilder()
  .setTitle('Title')
  .setDescription('Description (max 4096 chars)')
  .setColor(Colors.Blue)         // or 0x0099FF
  .setURL('https://example.com')
  .setAuthor({ name: 'Author', iconURL: '...', url: '...' })
  .setThumbnail('https://...')
  .setImage('https://...')
  .setTimestamp()
  .setFooter({ text: 'Footer', iconURL: '...' })
  .addFields(
    { name: 'Field 1', value: 'Value 1', inline: true },
    { name: 'Field 2', value: 'Value 2', inline: true },
  );

// Limits: 25 fields, 256 chars title, 4096 chars description, 6000 chars total
await interaction.reply({ embeds: [embed] });
```

## Buttons
```ts
import { ActionRowBuilder, ButtonBuilder, ButtonStyle } from 'discord.js';

const row = new ActionRowBuilder<ButtonBuilder>().addComponents(
  new ButtonBuilder().setCustomId('accept').setLabel('Accept').setStyle(ButtonStyle.Success),
  new ButtonBuilder().setCustomId('deny').setLabel('Deny').setStyle(ButtonStyle.Danger),
  new ButtonBuilder().setLabel('Link').setStyle(ButtonStyle.Link).setURL('https://...'),
);

const response = await interaction.reply({ content: 'Choose:', components: [row], withResponse: true });

// Collect
const collector = response.resource.message.createMessageComponentCollector({ time: 60_000 });
collector.on('collect', async (i) => {
  if (i.customId === 'accept') await i.update({ content: 'Accepted!', components: [] });
});
```

## Select Menus
```ts
import { StringSelectMenuBuilder, ActionRowBuilder } from 'discord.js';

const select = new StringSelectMenuBuilder()
  .setCustomId('starter')
  .setPlaceholder('Choose...')
  .addOptions(
    { label: 'Bulbasaur', value: 'bulbasaur', emoji: '🌿' },
    { label: 'Charmander', value: 'charmander', emoji: '🔥' },
  );

const row = new ActionRowBuilder().addComponents(select);
await interaction.reply({ components: [row] });
```

## Modals
```ts
import { ModalBuilder, TextInputBuilder, TextInputStyle, ActionRowBuilder } from 'discord.js';

const modal = new ModalBuilder().setCustomId('feedback').setTitle('Feedback');
const input = new TextInputBuilder()
  .setCustomId('message')
  .setLabel('Your feedback')
  .setStyle(TextInputStyle.Paragraph)  // or .Short
  .setRequired(true)
  .setMaxLength(1000);

modal.addComponents(new ActionRowBuilder<TextInputBuilder>().addComponents(input));
await interaction.showModal(modal);

// Handle submission
client.on(Events.InteractionCreate, async (i) => {
  if (!i.isModalSubmit()) return;
  const feedback = i.fields.getTextInputValue('message');
  await i.reply(`Received: ${feedback}`);
});
```

## Autocomplete
```ts
// In command builder
.addStringOption(opt => opt.setName('query').setDescription('Search').setAutocomplete(true))

// Handle
client.on(Events.InteractionCreate, async (i) => {
  if (!i.isAutocomplete()) return;
  const focused = i.options.getFocused();
  const choices = data.filter(c => c.startsWith(focused)).slice(0, 25);
  await i.respond(choices.map(c => ({ name: c, value: c })));
});
```

## Context Menus
```ts
import { ContextMenuCommandBuilder, ApplicationCommandType } from 'discord.js';

// User context menu
new ContextMenuCommandBuilder().setName('User Info').setType(ApplicationCommandType.User);

// Message context menu  
new ContextMenuCommandBuilder().setName('Report').setType(ApplicationCommandType.Message);

// Handle
if (interaction.isUserContextMenuCommand()) {
  const user = interaction.targetUser;
}
```

## Webhooks
```ts
import { WebhookClient } from 'discord.js';

const hook = new WebhookClient({ url: 'https://discord.com/api/webhooks/...' });
await hook.send({ content: 'Hello!', embeds: [embed], username: 'Bot Name' });
```

## Voice
```ts
import { joinVoiceChannel, createAudioPlayer, createAudioResource } from '@discordjs/voice';

const connection = joinVoiceChannel({
  channelId: channel.id,
  guildId: guild.id,
  adapterCreator: guild.voiceAdapterCreator,
});

const player = createAudioPlayer();
const resource = createAudioResource('audio.mp3');
player.play(resource);
connection.subscribe(player);
```

## Common Patterns
```ts
// Deferred reply (for slow operations)
await interaction.deferReply({ ephemeral: true });
await interaction.editReply('Done!');

// Ephemeral (only visible to user)
await interaction.reply({ content: 'Secret', ephemeral: true });

// Follow-up
await interaction.followUp('Additional info');

// Fetch member
const member = await interaction.guild.members.fetch(userId);

// Permissions check
if (!interaction.memberPermissions.has(PermissionFlagsBits.Administrator)) return;

// Collectors with filter
const filter = (i) => i.user.id === interaction.user.id;
const collected = await channel.awaitMessages({ filter, max: 1, time: 30_000 });
```

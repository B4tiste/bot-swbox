import discord
import requests
import os
from lib import infoRankSW, infoPlayerSwarena
from discord.ext import commands
from dotenv import load_dotenv

# Charger les variables d'environnement depuis le fichier .env
load_dotenv()

# Récupérer le token depuis les variables d'environnement
TOKEN = os.getenv('DISCORD_TOKEN')

# Initialisation du bot avec un préfixe (ex: !)
intents = discord.Intents.default()
intents.message_content = True  # Nécessaire pour lire le contenu des messages
bot = commands.Bot(command_prefix="!", intents=intents, help_command=None)

# Événement déclenché lorsque le bot est prêt
@bot.event
async def on_ready():
    print(f'Connecté en tant que {bot.user}')

# Enregistrer chaque commande exécutée par les utilisateurs
@bot.event
async def on_command(ctx):
    user = ctx.author
    command = ctx.command
    print(f"[LOG] Commande exécutée par {user}: !{command}")

# Charger tous les cogs au démarrage dans une fonction asynchrone
async def load_extensions():
    for filename in os.listdir('./cogs'):
        if filename.endswith('.py'):
            try:
                await bot.load_extension(f'cogs.{filename[:-3]}')
                print(f"Extension {filename} chargée.")
            except Exception as e:
                print(f"Erreur lors du chargement de l'extension {filename}: {e}")

# Commande pour recharger un cog dynamiquement
@bot.command()
@commands.is_owner()
async def reload(ctx, cog):
    try:
        await bot.reload_extension(f'cogs.{cog}')
        await ctx.send(f"Le cog `{cog}` a été rechargé avec succès.")
    except Exception as e:
        await ctx.send(f"Erreur lors du rechargement du cog {cog}: {str(e)}")

# Lancer le bot
async def main():
    async with bot:
        await load_extensions()  # Charger les cogs avant de démarrer le bot
        await bot.start(TOKEN)

# Exécuter le bot
import asyncio
asyncio.run(main())

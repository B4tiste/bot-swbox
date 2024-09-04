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

# Commande d'aide
@bot.command()
async def help(ctx):
    message = (
        "```"  # Bloc de code Markdown
        "Commandes disponibles:\n"
        "!ranks: Récupérer les scores des rangs\n"
        "!trackSwarena id <id>: Récupérer les saisons d'un joueur via son id\n"
        "!trackSwarena pseudo <pseudo>: Récupérer les saisons d'un joueur via son pseudo\n"
        "```"
    )
    await ctx.send(f"{ctx.author.mention}\n{message}")

# Commande pour récupérer les scores des rangs
@bot.command()
async def ranks(ctx):
    scores = infoRankSW()
    if scores:
        # Formatage du message avec Markdown pour Discord
        message = (
            "```"  # Bloc de code Markdown
            f"Rank    | Score\n"
            f"--------|-------\n"
            f"C1      | {scores['c1']}\n"
            f"C2      | {scores['c2']}\n"
            f"C3      | {scores['c3']}\n"
            f"P1      | {scores['p1']}\n"
            f"P2      | {scores['p2']}\n"
            f"P3      | {scores['p3']}\n"
            f"G1      | {scores['g1']}\n"
            f"G2      | {scores['g2']}\n"
            f"G3      | {scores['g3']}\n"
            "```"
        )
        await ctx.send(f"{ctx.author.mention}\n{message}")
    else:
        await ctx.send("Erreur lors de la récupération des données.")

# Commandes pour récupérer les saisons d'un joueur (id)
@bot.command()
async def trackSwarena(ctx, type: str, player):
    if type == "id":
        player_data = infoPlayerSwarena(player)
    elif type == "pseudo":
        url = "https://api.swarena.gg/player/search/" + player
        response = requests.get(url)
        data = response.json() # {"data":[{"wizard_name":"Falthazard","id":11934958}]}
        if "data" in data and data["data"]:
            player_id = data["data"][0]["id"]
            player_data = infoPlayerSwarena(player_id)
        else:
            player_data = None

    if player_data:
        for season in player_data:
            await ctx.send(f"Saison {season['season']}:\nNom: {season['name']}\nPays: {season['country']}\nPhoto URL: {season['picture']}")
    else:
        await ctx.send("Aucune saison disponible pour ce joueur.")

# Lancer le bot (insérer le token ici)
bot.run(TOKEN)

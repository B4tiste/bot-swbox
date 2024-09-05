from discord.ext import commands
from lib import infoRankSW, infoPlayerSwarena
import requests

class CommandsCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot
    
    # Commande d'aide
    @commands.command()
    async def help(self, ctx):
        message = (
            "```"  # Bloc de code Markdown
            "Commandes disponibles:\n"
            "!ranks: Récupérer les scores des rangs\n"
            "!trackSwarena id <id>: Récupérer les saisons d'un joueur via son id\n"
            "!trackSwarena pseudo <pseudo>: Récupérer les saisons d'un joueur via son pseudo\n"
            "```"
        )
        await ctx.send(f"{ctx.author.mention}\n{message}\n Bot créé par <@191619427584835585>")

    # Commande pour récupérer les scores des rangs
    @commands.command()
    async def ranks(self, ctx):
        scores = infoRankSW()
        if scores:
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

    # Commande pour récupérer les saisons d'un joueur via son id ou son pseudo
    @commands.command()
    async def trackSwarena(self, ctx, type: str, player):
        if type == "id":
            player_data = infoPlayerSwarena(player)
        elif type == "pseudo":
            url = "https://api.swarena.gg/player/search/" + player
            response = requests.get(url)
            data = response.json()
            if "data" in data and data["data"]:
                player_id = data["data"][0]["id"]
                player_data = infoPlayerSwarena(player_id)
            else:
                player_data = None

        if player_data:
            for season in player_data:
                await ctx.send(f"__Saison__ {season['season']}:\nNom: {season['name']}\nRank: {season['rank']}\nPays: {season['country']}\nPhoto URL: {season['picture']}")
        else:
            await ctx.send("Aucune saison disponible pour ce joueur.")

# Fonction nécessaire pour charger le cog
async def setup(bot):
    await bot.add_cog(CommandsCog(bot))
